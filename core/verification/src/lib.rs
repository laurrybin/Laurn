// Copyright 2026 laurrybin and Laurn Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use authority::AuthorityEngine;
use epoch::EpochEngine;
use policy::{EvaluationContext, Policy, PolicyEngine, PolicyRejectionReason, TransitionClass};
use commitment::StateCommitment;
use transition::Transition;

pub mod replay;

/// Maximum allowed size for a transition payload (4MB) to prevent memory DoS attacks.
pub const MAX_TRANSITION_PAYLOAD_SIZE: usize = 4 * 1024 * 1024;

/// The definitive result of evaluating a state transition through the Verification Engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationResult {
    /// The transition is perfectly valid and can be applied.
    Valid,
    /// The transition is mathematically or cryptographically invalid (tampered, bad signature, wrong output state).
    Invalid(&'static str),
    /// The transition violates strict timing or epoch boundaries (expired, future, too far ahead).
    Stale(&'static str),
    /// The transition references an unknown authority or epoch.
    Unknown(&'static str),
    /// The transition is fundamentally incompatible with the active global policy or authority capabilities.
    Incompatible(&'static str),
    /// The transition was already successfully verified and applied (duplicate/replay).
    Duplicate(&'static str),
    /// The transition was applied against a different state (preventing reordered execution).
    StateMismatch(&'static str),
}

/// The overarching Verification Engine that orchestrates all underlying domain engines
/// (Epoch, Authority, Policy, Transition, State) to produce a definitive verification result.
#[derive(Debug, Default)]
pub struct VerificationEngine;

/// A structured context containing all dependencies required to verify a transition.
#[derive(Debug)]
pub struct VerificationContext<'a> {
    pub transition: &'a Transition,
    pub raw_payload: &'a [u8],
    pub signature: &'a [u8; 64],
    pub expected_input_state: StateCommitment,
    pub generated_output_state: StateCommitment,
    pub authority_engine: &'a AuthorityEngine,
    pub epoch_engine: &'a EpochEngine,
    pub policy_engine: &'a PolicyEngine,
    pub policy: &'a Policy,
    pub seen_transitions: &'a replay::ReplayBuffer,
    pub parent_state_timestamp_ms: u64,
    pub has_evidence: bool,
    pub transition_protocol_version: u32,
    pub transition_class: TransitionClass,
}

impl VerificationEngine {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Extensively verifies a transition by sequentially running it through all LAURN engines.
    #[must_use]
    pub fn verify(&self, ctx: &VerificationContext) -> VerificationResult {
        
        // 0. Pre-Verification Integrity Checks (Replay & Ordering & Size)
        
        if ctx.raw_payload.len() > MAX_TRANSITION_PAYLOAD_SIZE {
            return VerificationResult::Invalid("Transition payload exceeds maximum allowed size");
        }

        if ctx.seen_transitions.contains(&ctx.transition.id) {
            return VerificationResult::Duplicate("Transition has already been applied");
        }

        if ctx.transition.input_state != ctx.expected_input_state {
            return VerificationResult::StateMismatch("Transition input state does not match expected active state");
        }

        // 1. Epoch Temporal Validation
        // Ensure the transition is assigned to the currently active epoch, and its timestamp is within bounds.
        if !ctx.epoch_engine.validate_transition_binding(
            &ctx.transition.metadata.epoch_id,
            ctx.transition.metadata.timestamp_ms,
        ) {
            return VerificationResult::Stale("Epoch is inactive, expired, or transition timestamp is out of bounds");
        }

        // 2. Authority Resolution
        let authority_id = &ctx.transition.metadata.authority_id;
        
        // We must check if the authority exists, and if so, what capabilities they have.
        // For strictness, if we can't fetch it, we fail. (The engine has check_capability, 
        // but we need to know if they exist at all before signature verification in a unified way, 
        // though verify_signature does this too. We will rely on verify_signature for existence 
        // and a subsequent capability check).

        // 3. Cryptographic Signature Validation
        // The signature MUST cover the entire serialized Transition struct to prevent cross-epoch replay attacks.
        let Ok(serialized_transition) = borsh::to_vec(ctx.transition) else {
            return VerificationResult::Invalid("Failed to serialize transition for signature verification");
        };

        if let Err(e) = ctx.authority_engine.verify_signature(authority_id, &serialized_transition, ctx.signature) {
            if e == "Unauthorized authority" {
                return VerificationResult::Unknown("Authority not registered");
            }
            return VerificationResult::Invalid("Cryptographic signature verification failed");
        }

        // 4. Policy Engine Evaluation
        let has_minimum_capability = ctx.authority_engine.check_capability(authority_id, ctx.policy.minimum_capability);
        let capabilities = if has_minimum_capability {
            ctx.policy.minimum_capability
        } else {
            authority::AuthorityCapability::empty()
        };

        let eval_ctx = EvaluationContext {
            transition_protocol_version: ctx.transition_protocol_version,
            metadata: &ctx.transition.metadata,
            parent_state_timestamp_ms: ctx.parent_state_timestamp_ms,
            has_evidence: ctx.has_evidence,
            transition_class: ctx.transition_class,
            authority_capabilities: capabilities,
        };

        let policy_decision = ctx.policy_engine.evaluate(ctx.policy, &eval_ctx);

        match policy_decision {
            policy::PolicyDecision::Accepted => {}
            policy::PolicyDecision::Rejected(reason) => {
                return match reason {
                    PolicyRejectionReason::ProtocolVersionMismatch => VerificationResult::Incompatible("Protocol version mismatch"),
                    PolicyRejectionReason::StateFreshnessViolation => VerificationResult::Stale("State freshness threshold exceeded"),
                    PolicyRejectionReason::EvidenceMissing => VerificationResult::Incompatible("Required evidence missing"),
                    PolicyRejectionReason::TransitionClassNotAllowed => VerificationResult::Incompatible("Transition class not permitted by policy"),
                    PolicyRejectionReason::InsufficientAuthorityCapability => VerificationResult::Incompatible("Authority lacks required capability"),
                };
            }
        }

        // 5. Transition Integrity Validation
        // Ensures the raw payload matches the commitment, and the output state matches what was generated.
        if !ctx.transition.validate(ctx.raw_payload, ctx.generated_output_state) {
            return VerificationResult::Invalid("Transition integrity check failed (payload mismatch or output state mismatch)");
        }

        VerificationResult::Valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use authority::{Authority, AuthorityCapability, AuthorityId, AuthorityRole};
    use epoch::{Epoch, EpochId, EpochStatus};
    use transition::{TransitionCommitment, TransitionId, TransitionMetadata};
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    fn generate_keypair() -> (SigningKey, AuthorityId) {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        (signing_key, AuthorityId(verifying_key.to_bytes()))
    }

    fn setup_environment() -> (
        AuthorityEngine,
        EpochEngine,
        PolicyEngine,
        Policy,
        SigningKey,
        AuthorityId,
        EpochId,
    ) {
        let mut auth_engine = AuthorityEngine::new();
        let mut epoch_engine = EpochEngine::new();
        let policy_engine = PolicyEngine::new();

        let (signing_key, auth_id) = generate_keypair();

        let authority = Authority {
            id: auth_id,
            role: AuthorityRole::Client,
            capabilities: AuthorityCapability::CAN_SUBMIT_TRANSITION | AuthorityCapability::CAN_SPAWN,
            session_binding: None,
        };
        auth_engine.register_authority(authority).unwrap();

        let epoch_id = EpochId([1u8; 32]);
        let epoch = Epoch {
            id: epoch_id,
            sequence: 1,
            start_time_ms: 1000,
            expiration_time_ms: 2000,
            status: EpochStatus::Pending,
            initial_state: StateCommitment([0u8; 32]),
        };
        epoch_engine.register_epoch(epoch).unwrap();
        epoch_engine.activate_epoch(epoch_id).unwrap();

        let policy = Policy {
            protocol_version: 1,
            max_state_freshness_ms: 500,
            require_evidence: true,
            allowed_transition_classes: TransitionClass::INPUT | TransitionClass::SPAWN,
            minimum_capability: AuthorityCapability::CAN_SUBMIT_TRANSITION,
        };

        (
            auth_engine,
            epoch_engine,
            policy_engine,
            policy,
            signing_key,
            auth_id,
            epoch_id,
        )
    }

    fn create_valid_transition(
        auth_id: AuthorityId,
        epoch_id: EpochId,
        timestamp_ms: u64,
        raw_payload: &[u8],
    ) -> Transition {
        Transition {
            id: TransitionId(1),
            metadata: TransitionMetadata {
                authority_id: auth_id,
                epoch_id,
                timestamp_ms,
            },
            input_state: StateCommitment([0u8; 32]),
            output_state: StateCommitment([2u8; 32]),
            payload_commitment: TransitionCommitment::compute(raw_payload),
        }
    }

    #[test]
    fn test_valid_transition() {
        let (auth_engine, epoch_engine, policy_engine, policy, signing_key, auth_id, epoch_id) =
            setup_environment();

        let raw_payload = b"move_forward";
        let transition = create_valid_transition(auth_id, epoch_id, 1500, raw_payload);
        
        let serialized_transition = borsh::to_vec(&transition).unwrap();
        let signature = signing_key.sign(&serialized_transition);

        let engine = VerificationEngine::new();
        let sig_bytes = signature.to_bytes();
        let ctx = VerificationContext {
            transition: &transition,
            raw_payload,
            signature: &sig_bytes,
            expected_input_state: StateCommitment([0u8; 32]),
            seen_transitions: &crate::replay::ReplayBuffer::default(),
            generated_output_state: StateCommitment([2u8; 32]), // Matching output state
            authority_engine: &auth_engine,
            epoch_engine: &epoch_engine,
            policy_engine: &policy_engine,
            policy: &policy,
            parent_state_timestamp_ms: 1400, // Parent state timestamp (within 500ms freshness)
            has_evidence: true,              // Has evidence
            transition_protocol_version: 1,  // Protocol version matches
            transition_class: TransitionClass::INPUT,
        };

        let result = engine.verify(&ctx);

        assert_eq!(result, VerificationResult::Valid);
    }

    #[test]
    fn test_invalid_signature() {
        let (auth_engine, epoch_engine, policy_engine, policy, signing_key, auth_id, epoch_id) =
            setup_environment();

        let raw_payload = b"move_forward";
        let transition = create_valid_transition(auth_id, epoch_id, 1500, raw_payload);
        
        // We sign a DIFFERENT transition payload
        let mut tampered_transition = transition.clone();
        tampered_transition.id = TransitionId(999); // Tampered!
        let serialized_tampered = borsh::to_vec(&tampered_transition).unwrap();
        
        // Sign the tampered data, but submit the original transition. Signature will fail.
        let signature = signing_key.sign(&serialized_tampered);

        let engine = VerificationEngine::new();
        let sig_bytes = signature.to_bytes();
        let ctx = VerificationContext {
            transition: &transition,
            raw_payload,
            signature: &sig_bytes,
            expected_input_state: StateCommitment([0u8; 32]),
            seen_transitions: &crate::replay::ReplayBuffer::default(),
            generated_output_state: StateCommitment([2u8; 32]),
            authority_engine: &auth_engine,
            epoch_engine: &epoch_engine,
            policy_engine: &policy_engine,
            policy: &policy,
            parent_state_timestamp_ms: 1400,
            has_evidence: true,
            transition_protocol_version: 1,
            transition_class: TransitionClass::INPUT,
        };

        let result = engine.verify(&ctx);

        assert_eq!(result, VerificationResult::Invalid("Cryptographic signature verification failed"));
    }

    #[test]
    fn test_stale_epoch() {
        let (auth_engine, epoch_engine, policy_engine, policy, signing_key, auth_id, epoch_id) =
            setup_environment();

        let raw_payload = b"move_forward";
        // Epoch expires at 2000. 2500 is out of bounds.
        let transition = create_valid_transition(auth_id, epoch_id, 2500, raw_payload);
        let serialized_transition = borsh::to_vec(&transition).unwrap();
        let signature = signing_key.sign(&serialized_transition);

        let engine = VerificationEngine::new();
        let sig_bytes = signature.to_bytes();
        let ctx = VerificationContext {
            transition: &transition,
            raw_payload,
            signature: &sig_bytes,
            expected_input_state: StateCommitment([0u8; 32]),
            seen_transitions: &crate::replay::ReplayBuffer::default(),
            generated_output_state: StateCommitment([2u8; 32]),
            authority_engine: &auth_engine,
            epoch_engine: &epoch_engine,
            policy_engine: &policy_engine,
            policy: &policy,
            parent_state_timestamp_ms: 2400,
            has_evidence: true,
            transition_protocol_version: 1,
            transition_class: TransitionClass::INPUT,
        };

        let result = engine.verify(&ctx);

        assert_eq!(result, VerificationResult::Stale("Epoch is inactive, expired, or transition timestamp is out of bounds"));
    }

    #[test]
    fn test_unknown_authority() {
        let (auth_engine, epoch_engine, policy_engine, policy, _, _, epoch_id) =
            setup_environment();

        let (unregistered_key, unregistered_id) = generate_keypair();

        let raw_payload = b"move_forward";
        let transition = create_valid_transition(unregistered_id, epoch_id, 1500, raw_payload);
        let serialized_transition = borsh::to_vec(&transition).unwrap();
        let signature = unregistered_key.sign(&serialized_transition);

        let engine = VerificationEngine::new();
        let sig_bytes = signature.to_bytes();
        let ctx = VerificationContext {
            transition: &transition,
            raw_payload,
            signature: &sig_bytes,
            expected_input_state: StateCommitment([0u8; 32]),
            seen_transitions: &crate::replay::ReplayBuffer::default(),
            generated_output_state: StateCommitment([2u8; 32]),
            authority_engine: &auth_engine, // Engine doesn't know about this authority
            epoch_engine: &epoch_engine,
            policy_engine: &policy_engine,
            policy: &policy,
            parent_state_timestamp_ms: 1400,
            has_evidence: true,
            transition_protocol_version: 1,
            transition_class: TransitionClass::INPUT,
        };

        let result = engine.verify(&ctx);

        assert_eq!(result, VerificationResult::Unknown("Authority not registered"));
    }

    #[test]
    fn test_incompatible_policy() {
        let (auth_engine, epoch_engine, policy_engine, policy, signing_key, auth_id, epoch_id) =
            setup_environment();

        let raw_payload = b"move_forward";
        let transition = create_valid_transition(auth_id, epoch_id, 1500, raw_payload);
        let serialized_transition = borsh::to_vec(&transition).unwrap();
        let signature = signing_key.sign(&serialized_transition);

        let engine = VerificationEngine::new();
        let sig_bytes = signature.to_bytes();
        let ctx = VerificationContext {
            transition: &transition,
            raw_payload,
            signature: &sig_bytes,
            expected_input_state: StateCommitment([0u8; 32]),
            seen_transitions: &crate::replay::ReplayBuffer::default(),
            generated_output_state: StateCommitment([2u8; 32]),
            authority_engine: &auth_engine,
            epoch_engine: &epoch_engine,
            policy_engine: &policy_engine,
            policy: &policy,
            parent_state_timestamp_ms: 1400,
            has_evidence: false, // MISSING EVIDENCE! Policy requires evidence.
            transition_protocol_version: 1,
            transition_class: TransitionClass::INPUT,
        };

        let result = engine.verify(&ctx);

        assert_eq!(result, VerificationResult::Incompatible("Required evidence missing"));
    }

    #[test]
    fn test_attack_modified_state() {
        let (auth_engine, epoch_engine, policy_engine, policy, signing_key, auth_id, epoch_id) =
            setup_environment();

        let raw_payload = b"move_forward";
        let transition = create_valid_transition(auth_id, epoch_id, 1500, raw_payload);
        let serialized_transition = borsh::to_vec(&transition).unwrap();
        let signature = signing_key.sign(&serialized_transition);

        let engine = VerificationEngine::new();
        let sig_bytes = signature.to_bytes();
        
        let ctx = VerificationContext {
            transition: &transition,
            raw_payload,
            signature: &sig_bytes,
            expected_input_state: StateCommitment([0u8; 32]),
            seen_transitions: &crate::replay::ReplayBuffer::default(),
            generated_output_state: StateCommitment([99u8; 32]), // TAMPERED OUTPUT STATE
            authority_engine: &auth_engine,
            epoch_engine: &epoch_engine,
            policy_engine: &policy_engine,
            policy: &policy,
            parent_state_timestamp_ms: 1400,
            has_evidence: true,
            transition_protocol_version: 1,
            transition_class: TransitionClass::INPUT,
        };

        let result = engine.verify(&ctx);
        assert_eq!(result, VerificationResult::Invalid("Transition integrity check failed (payload mismatch or output state mismatch)"));
    }

    #[test]
    fn test_attack_reordered_transition() {
        let (auth_engine, epoch_engine, policy_engine, policy, signing_key, auth_id, epoch_id) =
            setup_environment();

        let raw_payload = b"move_forward";
        let transition = create_valid_transition(auth_id, epoch_id, 1500, raw_payload);
        let serialized_transition = borsh::to_vec(&transition).unwrap();
        let signature = signing_key.sign(&serialized_transition);

        let engine = VerificationEngine::new();
        let sig_bytes = signature.to_bytes();
        
        let ctx = VerificationContext {
            transition: &transition,
            raw_payload,
            signature: &sig_bytes,
            expected_input_state: StateCommitment([1u8; 32]), // EXPECTED INPUT IS DIFFERENT FROM TRANSITION INPUT!
            seen_transitions: &crate::replay::ReplayBuffer::default(),
            generated_output_state: StateCommitment([2u8; 32]),
            authority_engine: &auth_engine,
            epoch_engine: &epoch_engine,
            policy_engine: &policy_engine,
            policy: &policy,
            parent_state_timestamp_ms: 1400,
            has_evidence: true,
            transition_protocol_version: 1,
            transition_class: TransitionClass::INPUT,
        };

        let result = engine.verify(&ctx);
        assert_eq!(result, VerificationResult::StateMismatch("Transition input state does not match expected active state"));
    }

    #[test]
    fn test_attack_duplicated_transition() {
        let (auth_engine, epoch_engine, policy_engine, policy, signing_key, auth_id, epoch_id) =
            setup_environment();

        let raw_payload = b"move_forward";
        let transition = create_valid_transition(auth_id, epoch_id, 1500, raw_payload);
        let serialized_transition = borsh::to_vec(&transition).unwrap();
        let signature = signing_key.sign(&serialized_transition);

        let engine = VerificationEngine::new();
        let sig_bytes = signature.to_bytes();
        
        let mut seen = crate::replay::ReplayBuffer::default();
        seen.insert(transition.id); // TRANSITION WAS ALREADY SEEN!

        let ctx = VerificationContext {
            transition: &transition,
            raw_payload,
            signature: &sig_bytes,
            expected_input_state: StateCommitment([0u8; 32]),
            seen_transitions: &seen,
            generated_output_state: StateCommitment([2u8; 32]),
            authority_engine: &auth_engine,
            epoch_engine: &epoch_engine,
            policy_engine: &policy_engine,
            policy: &policy,
            parent_state_timestamp_ms: 1400,
            has_evidence: true,
            transition_protocol_version: 1,
            transition_class: TransitionClass::INPUT,
        };

        let result = engine.verify(&ctx);
        assert_eq!(result, VerificationResult::Duplicate("Transition has already been applied"));
    }

    #[test]
    fn test_attack_epoch_substitution() {
        let (auth_engine, epoch_engine, policy_engine, policy, signing_key, auth_id, _) =
            setup_environment();

        let raw_payload = b"move_forward";
        // USE AN INVALID EPOCH
        let invalid_epoch_id = EpochId([99u8; 32]);
        let transition = create_valid_transition(auth_id, invalid_epoch_id, 1500, raw_payload);
        let serialized_transition = borsh::to_vec(&transition).unwrap();
        let signature = signing_key.sign(&serialized_transition);

        let engine = VerificationEngine::new();
        let sig_bytes = signature.to_bytes();
        
        let ctx = VerificationContext {
            transition: &transition,
            raw_payload,
            signature: &sig_bytes,
            expected_input_state: StateCommitment([0u8; 32]),
            seen_transitions: &crate::replay::ReplayBuffer::default(),
            generated_output_state: StateCommitment([2u8; 32]),
            authority_engine: &auth_engine,
            epoch_engine: &epoch_engine,
            policy_engine: &policy_engine,
            policy: &policy,
            parent_state_timestamp_ms: 1400,
            has_evidence: true,
            transition_protocol_version: 1,
            transition_class: TransitionClass::INPUT,
        };

        let result = engine.verify(&ctx);
        assert_eq!(result, VerificationResult::Stale("Epoch is inactive, expired, or transition timestamp is out of bounds"));
    }

    #[test]
    fn test_attack_malformed_payload() {
        let (auth_engine, epoch_engine, policy_engine, policy, signing_key, auth_id, epoch_id) =
            setup_environment();

        let raw_payload = b"move_forward";
        let transition = create_valid_transition(auth_id, epoch_id, 1500, raw_payload);
        let serialized_transition = borsh::to_vec(&transition).unwrap();
        let signature = signing_key.sign(&serialized_transition);

        let engine = VerificationEngine::new();
        let sig_bytes = signature.to_bytes();
        
        let malformed_payload = b"move_backward"; // MALFORMED PAYLOAD SUPPLIED!

        let ctx = VerificationContext {
            transition: &transition,
            raw_payload: malformed_payload,
            signature: &sig_bytes,
            expected_input_state: StateCommitment([0u8; 32]),
            seen_transitions: &crate::replay::ReplayBuffer::default(),
            generated_output_state: StateCommitment([2u8; 32]),
            authority_engine: &auth_engine,
            epoch_engine: &epoch_engine,
            policy_engine: &policy_engine,
            policy: &policy,
            parent_state_timestamp_ms: 1400,
            has_evidence: true,
            transition_protocol_version: 1,
            transition_class: TransitionClass::INPUT,
        };

        let result = engine.verify(&ctx);
        assert_eq!(result, VerificationResult::Invalid("Transition integrity check failed (payload mismatch or output state mismatch)"));
    }
}

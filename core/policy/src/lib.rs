// Copyright 2026 Darwin Clay O. and Lawrence Obina
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

use authority::AuthorityCapability;
use bitflags::bitflags;
use borsh::{BorshDeserialize, BorshSerialize};
use transition::TransitionMetadata;

bitflags! {
    /// Categorizes the type of a transition to enforce policy limits dynamically.
    /// Uses a u32 bitmask for fast bitwise checks and C-FFI friendliness.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct TransitionClass: u32 {
        /// Standard user input (e.g., movement, action).
        const INPUT = 1 << 0;
        /// Spawning or destruction of an entity.
        const SPAWN = 1 << 1;
        /// Explicit mutation of time or epoch boundaries.
        const TIME_CONTROL = 1 << 2;
        /// High-priority administrative action.
        const ADMIN = 1 << 31;
    }
}

impl BorshSerialize for TransitionClass {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.bits().serialize(writer)
    }
}

impl BorshDeserialize for TransitionClass {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bits = u32::deserialize_reader(reader)?;
        TransitionClass::from_bits(bits)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid bitflags"))
    }
}

/// A versioned deterministic policy dictating the global rules for accepting state transitions.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Policy {
    /// The strict protocol version expected by the network.
    pub protocol_version: u32,
    /// Maximum allowed delta in milliseconds between a transition's timestamp
    /// and the timestamp of the state it is applied to (prevents applying transitions to ancient states).
    pub max_state_freshness_ms: u64,
    /// Whether transitions must be accompanied by cryptographically verified evidence.
    pub require_evidence: bool,
    /// A bitmask of transition classes currently allowed.
    /// Can be used to lock down the simulation (e.g. pause SPAWN).
    pub allowed_transition_classes: TransitionClass,
    /// Minimum authority capability required to submit any transition under this policy.
    pub minimum_capability: AuthorityCapability,
}

/// The result of evaluating a transition against the active policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    Accepted,
    Rejected(PolicyRejectionReason),
}

/// Specific deterministic reasons for a policy rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyRejectionReason {
    ProtocolVersionMismatch,
    StateFreshnessViolation,
    EvidenceMissing,
    TransitionClassNotAllowed,
    InsufficientAuthorityCapability,
}

/// A context structure to group arguments for policy evaluation.
#[derive(Debug, Clone)]
pub struct EvaluationContext<'a> {
    pub transition_protocol_version: u32,
    pub metadata: &'a TransitionMetadata,
    pub parent_state_timestamp_ms: u64,
    pub has_evidence: bool,
    pub transition_class: TransitionClass,
    pub authority_capabilities: AuthorityCapability,
}

/// The deterministic engine for evaluating transitions against a global policy.
#[derive(Debug, Default)]
pub struct PolicyEngine;

impl PolicyEngine {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Evaluates a transition context against the provided policy.
    #[must_use]
    pub fn evaluate(&self, policy: &Policy, ctx: &EvaluationContext) -> PolicyDecision {
        // 1. Protocol Version
        if ctx.transition_protocol_version != policy.protocol_version {
            return PolicyDecision::Rejected(PolicyRejectionReason::ProtocolVersionMismatch);
        }

        // 2. Evidence Requirements
        if policy.require_evidence && !ctx.has_evidence {
            return PolicyDecision::Rejected(PolicyRejectionReason::EvidenceMissing);
        }

        // 3. State Freshness (Delta between transition and parent state)
        if ctx.metadata.timestamp_ms >= ctx.parent_state_timestamp_ms {
            let delta = ctx.metadata.timestamp_ms - ctx.parent_state_timestamp_ms;
            if delta > policy.max_state_freshness_ms {
                return PolicyDecision::Rejected(PolicyRejectionReason::StateFreshnessViolation);
            }
        } else {
            // A transition timestamp cannot logically pre-date the state it is applied against
            return PolicyDecision::Rejected(PolicyRejectionReason::StateFreshnessViolation);
        }

        // 4. Transition Class Allowance
        if !policy
            .allowed_transition_classes
            .contains(ctx.transition_class)
        {
            return PolicyDecision::Rejected(PolicyRejectionReason::TransitionClassNotAllowed);
        }

        // 5. Authority Capability Requirement
        if !ctx
            .authority_capabilities
            .contains(policy.minimum_capability)
        {
            return PolicyDecision::Rejected(
                PolicyRejectionReason::InsufficientAuthorityCapability,
            );
        }

        PolicyDecision::Accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use authority::AuthorityId;
    use epoch::EpochId;

    fn generate_test_metadata(timestamp_ms: u64) -> TransitionMetadata {
        TransitionMetadata {
            authority_id: AuthorityId([0u8; 32]),
            epoch_id: EpochId([1u8; 32]),
            timestamp_ms,
        }
    }

    fn default_policy() -> Policy {
        Policy {
            protocol_version: 1,
            max_state_freshness_ms: 500, // 500ms max age
            require_evidence: true,
            allowed_transition_classes: TransitionClass::INPUT | TransitionClass::SPAWN,
            minimum_capability: AuthorityCapability::CAN_SUBMIT_TRANSITION,
        }
    }

    #[test]
    fn test_policy_acceptance() {
        let engine = PolicyEngine::new();
        let policy = default_policy();
        let metadata = generate_test_metadata(1000);
        let ctx = EvaluationContext {
            transition_protocol_version: 1, // matching version
            metadata: &metadata,
            parent_state_timestamp_ms: 900, // Delta is 100ms <= 500ms
            has_evidence: true,             // has evidence
            transition_class: TransitionClass::INPUT,
            authority_capabilities: AuthorityCapability::CAN_SUBMIT_TRANSITION
                | AuthorityCapability::CAN_SPAWN,
        };

        let decision = engine.evaluate(&policy, &ctx);

        assert_eq!(decision, PolicyDecision::Accepted);
    }

    #[test]
    fn test_policy_version_mismatch() {
        let engine = PolicyEngine::new();
        let policy = default_policy();
        let metadata = generate_test_metadata(1000);
        let ctx = EvaluationContext {
            transition_protocol_version: 2, // mismatch!
            metadata: &metadata,
            parent_state_timestamp_ms: 900,
            has_evidence: true,
            transition_class: TransitionClass::INPUT,
            authority_capabilities: AuthorityCapability::CAN_SUBMIT_TRANSITION,
        };

        let decision = engine.evaluate(&policy, &ctx);

        assert_eq!(
            decision,
            PolicyDecision::Rejected(PolicyRejectionReason::ProtocolVersionMismatch)
        );
    }

    #[test]
    fn test_policy_evidence_missing() {
        let engine = PolicyEngine::new();
        let policy = default_policy();
        let metadata = generate_test_metadata(1000);
        let ctx = EvaluationContext {
            transition_protocol_version: 1,
            metadata: &metadata,
            parent_state_timestamp_ms: 900,
            has_evidence: false, // Missing evidence!
            transition_class: TransitionClass::INPUT,
            authority_capabilities: AuthorityCapability::CAN_SUBMIT_TRANSITION,
        };

        let decision = engine.evaluate(&policy, &ctx);

        assert_eq!(
            decision,
            PolicyDecision::Rejected(PolicyRejectionReason::EvidenceMissing)
        );
    }

    #[test]
    fn test_policy_state_freshness_violation() {
        let engine = PolicyEngine::new();
        let policy = default_policy();

        // Test transition is too far ahead of parent state
        let metadata = generate_test_metadata(1600);
        let ctx1 = EvaluationContext {
            transition_protocol_version: 1,
            metadata: &metadata,
            parent_state_timestamp_ms: 1000, // Delta 600ms > 500ms
            has_evidence: true,
            transition_class: TransitionClass::INPUT,
            authority_capabilities: AuthorityCapability::CAN_SUBMIT_TRANSITION,
        };
        let decision = engine.evaluate(&policy, &ctx1);
        assert_eq!(
            decision,
            PolicyDecision::Rejected(PolicyRejectionReason::StateFreshnessViolation)
        );

        // Test transition predates parent state
        let predates_metadata = generate_test_metadata(900);
        let ctx2 = EvaluationContext {
            transition_protocol_version: 1,
            metadata: &predates_metadata,
            parent_state_timestamp_ms: 1000,
            has_evidence: true,
            transition_class: TransitionClass::INPUT,
            authority_capabilities: AuthorityCapability::CAN_SUBMIT_TRANSITION,
        };
        let decision2 = engine.evaluate(&policy, &ctx2);
        assert_eq!(
            decision2,
            PolicyDecision::Rejected(PolicyRejectionReason::StateFreshnessViolation)
        );
    }

    #[test]
    fn test_policy_transition_class_not_allowed() {
        let engine = PolicyEngine::new();
        let policy = default_policy(); // Only INPUT and SPAWN allowed
        let metadata = generate_test_metadata(1000);
        let ctx = EvaluationContext {
            transition_protocol_version: 1,
            metadata: &metadata,
            parent_state_timestamp_ms: 900,
            has_evidence: true,
            transition_class: TransitionClass::TIME_CONTROL, // Not allowed!
            authority_capabilities: AuthorityCapability::CAN_SUBMIT_TRANSITION
                | AuthorityCapability::CAN_AUTHORIZE_TIME,
        };

        let decision = engine.evaluate(&policy, &ctx);

        assert_eq!(
            decision,
            PolicyDecision::Rejected(PolicyRejectionReason::TransitionClassNotAllowed)
        );
    }

    #[test]
    fn test_policy_insufficient_authority_capability() {
        let engine = PolicyEngine::new();
        let policy = default_policy(); // Requires CAN_SUBMIT_TRANSITION
        let metadata = generate_test_metadata(1000);

        let ctx = EvaluationContext {
            transition_protocol_version: 1,
            metadata: &metadata,
            parent_state_timestamp_ms: 900,
            has_evidence: true,
            transition_class: TransitionClass::INPUT,
            authority_capabilities: AuthorityCapability::empty(), // Insufficient!
        };
        let decision = engine.evaluate(&policy, &ctx);

        assert_eq!(
            decision,
            PolicyDecision::Rejected(PolicyRejectionReason::InsufficientAuthorityCapability)
        );
    }
}

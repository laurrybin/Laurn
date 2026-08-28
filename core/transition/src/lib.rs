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

use authority::AuthorityId;
use borsh::{BorshDeserialize, BorshSerialize};
use commitment::{CommitmentEngine, StateCommitment, TRANSITION_DOMAIN_V1};
use epoch::EpochId;
use subtle::{Choice, ConstantTimeEq};

/// `TransitionCommitment` is a deterministic, canonical representation of a state transition payload.
/// It is represented as a canonical 32-byte cryptographic commitment (e.g. BLAKE3).
#[derive(Debug, Clone, Copy, Eq, Hash, BorshSerialize, BorshDeserialize)]
#[allow(clippy::derived_hash_with_manual_eq)]
pub struct TransitionCommitment(pub [u8; 32]);

impl TransitionCommitment {
    /// Returns a reference to the underlying byte array.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Computes a `TransitionCommitment` from the provided canonical bytes.
    #[must_use]
    pub fn compute(bytes: &[u8]) -> Self {
        Self(CommitmentEngine::compute(TRANSITION_DOMAIN_V1, bytes))
    }
}

// Implement constant-time equality to prevent timing attacks during verification.
impl ConstantTimeEq for TransitionCommitment {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.0.ct_eq(&other.0)
    }
}

impl PartialEq for TransitionCommitment {
    fn eq(&self, other: &Self) -> bool {
        self.ct_eq(other).into()
    }
}

/// A unique identifier for a Transition within an Epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
#[repr(transparent)]
pub struct TransitionId(pub u64);

/// Metadata associated with a Transition, carrying authority and temporal information.
/// Timestamps are strictly represented as `u64` (e.g., milliseconds since Unix Epoch)
/// to avoid floating point non-determinism across platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct TransitionMetadata {
    pub authority_id: AuthorityId,
    pub epoch_id: EpochId,
    pub timestamp_ms: u64,
}

/// A verifiable transition linking `State[n]` to `State[n+1]`.
///
/// This struct holds the cryptographic commitments of the inputs and outputs, rather than
/// the raw payloads, keeping it lightweight.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Transition {
    pub id: TransitionId,
    pub metadata: TransitionMetadata,
    pub input_state: StateCommitment,
    pub output_state: StateCommitment,
    pub payload_commitment: TransitionCommitment,
}

impl Transition {
    /// Validates the transition by proving that the provided raw payload matches the `payload_commitment`
    /// and that executing the payload on `input_state` legitimately yields `output_state`.
    ///
    /// In Phase 05, this simply validates the cryptographic integrity of the payload commitment
    /// and the matching generated output state. Future phases will integrate the WASM execution engine here.
    #[must_use]
    pub fn validate(&self, raw_payload: &[u8], generated_output_state: StateCommitment) -> bool {
        // 1. Verify the payload matches the commitment
        let computed_payload_commitment = TransitionCommitment::compute(raw_payload);
        if self.payload_commitment != computed_payload_commitment {
            return false;
        }

        // 2. Verify the output state matches what the engine actually produced
        if self.output_state != generated_output_state {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_commitment_constant_time_eq() {
        let a = TransitionCommitment([0u8; 32]);
        let mut b_bytes = [0u8; 32];
        b_bytes[0] = 1;
        let b = TransitionCommitment(b_bytes);

        assert_eq!(a, a);
        assert_ne!(a, b);

        let choice: Choice = a.ct_eq(&a);
        assert!(bool::from(choice));
    }

    #[test]
    fn test_transition_validation() {
        let raw_payload = b"jump action";
        let valid_payload_commitment = TransitionCommitment::compute(raw_payload);

        let input_state = StateCommitment([1u8; 32]);
        let output_state = StateCommitment([2u8; 32]);

        let metadata = TransitionMetadata {
            authority_id: AuthorityId([0u8; 32]),
            epoch_id: EpochId([1u8; 32]),
            timestamp_ms: 1622548800000,
        };

        let transition = Transition {
            id: TransitionId(42),
            metadata,
            input_state,
            output_state,
            payload_commitment: valid_payload_commitment,
        };

        // Pass validation with correct payload and expected output
        assert!(transition.validate(raw_payload, output_state));

        // Fail validation with tampered payload
        let tampered_payload = b"shoot action";
        assert!(!transition.validate(tampered_payload, output_state));

        // Fail validation with tampered output state
        let tampered_output_state = StateCommitment([3u8; 32]);
        assert!(!transition.validate(raw_payload, tampered_output_state));
    }
}

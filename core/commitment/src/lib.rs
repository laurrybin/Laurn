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

use borsh::{BorshDeserialize, BorshSerialize};
use subtle::{Choice, ConstantTimeEq};

/// The domain separator used when committing to a state.
pub const STATE_DOMAIN_V1: &[u8] = b"LAURN_STATE_V1";

/// The domain separator used when committing to a transition.
pub const TRANSITION_DOMAIN_V1: &[u8] = b"LAURN_TRANS_V1";

/// `StateCommitment` is a deterministic, canonical representation of a simulation state.
/// It is represented as a canonical 32-byte cryptographic commitment (e.g. BLAKE3).
#[derive(Debug, Clone, Copy, Eq, Hash, BorshSerialize, BorshDeserialize)]
#[allow(clippy::derived_hash_with_manual_eq)]
pub struct StateCommitment(pub [u8; 32]);

impl StateCommitment {
    /// Returns a reference to the underlying byte array.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

// Implement constant-time equality to prevent timing attacks during verification.
impl ConstantTimeEq for StateCommitment {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.0.ct_eq(&other.0)
    }
}

impl PartialEq for StateCommitment {
    fn eq(&self, other: &Self) -> bool {
        self.ct_eq(other).into()
    }
}

/// The `CommitmentEngine` computes secure 32-byte hashes for LAURN primitives.
pub struct CommitmentEngine;

impl CommitmentEngine {
    /// Computes a hash given a specific domain separator and canonical bytes.
    #[must_use]
    pub fn compute(domain_separator: &[u8], bytes: &[u8]) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        // Mix the domain separator into the hash context first
        hasher.update(domain_separator);
        hasher.update(bytes);
        *hasher.finalize().as_bytes()
    }

    /// Computes a `StateCommitment` for the provided canonical bytes.
    #[must_use]
    pub fn commit_state(bytes: &[u8]) -> StateCommitment {
        StateCommitment(Self::compute(STATE_DOMAIN_V1, bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_separation() {
        let payload = b"example payload";
        let state_hash = CommitmentEngine::compute(STATE_DOMAIN_V1, payload);
        let trans_hash = CommitmentEngine::compute(TRANSITION_DOMAIN_V1, payload);

        // Identical bytes must yield different hashes due to domain separation
        assert_ne!(state_hash, trans_hash);
    }

    #[test]
    fn test_constant_time_eq() {
        let a = StateCommitment([0u8; 32]);
        let mut b_bytes = [0u8; 32];
        let mut c_bytes = [0u8; 32];
        b_bytes[0] = 1;
        c_bytes[31] = 1;

        let b = StateCommitment(b_bytes);
        let c = StateCommitment(c_bytes);

        assert_eq!(a, a);
        assert_ne!(a, b);
        assert_ne!(a, c);

        let choice: Choice = a.ct_eq(&a);
        assert!(bool::from(choice));
    }

    #[test]
    fn test_mutation_avalanche() {
        let payload1 = b"test payload 1";
        let payload2 = b"test payload 2"; // Only 1 character different

        let hash1 = CommitmentEngine::compute(STATE_DOMAIN_V1, payload1);
        let hash2 = CommitmentEngine::compute(STATE_DOMAIN_V1, payload2);

        assert_ne!(hash1, hash2);
    }
}

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
use commitment::StateCommitment;
use epoch::EpochId;
use version_crate::ProtocolVersion;

/// `StateId` uniquely identifies a specific state representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
pub struct StateId(pub [u8; 32]);

/// `ParentStateId` links a state to its predecessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
pub struct ParentStateId(pub [u8; 32]);

/// `StateVersion` represents the monotonic version number of the state.
pub type StateVersion = u64;

/// `SimulationTick` represents the logical time of the simulation.
pub type SimulationTick = u64;

/// `CanonicalState` models the conceptually agreed state envelope.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct CanonicalState {
    pub state_id: StateId,
    pub parent_state_id: Option<ParentStateId>,
    pub epoch_id: EpochId,
    pub authority_id: AuthorityId,
    pub state_version: StateVersion,
    pub simulation_tick: SimulationTick,
    pub state_commitment: StateCommitment,
    pub protocol_version: ProtocolVersion,
}

/// A trait for explicit state ownership and domain registration.
/// Only state within this boundary participates in deterministic commitments.
pub trait DeterministicStateDomain {
    /// Returns the canonical, deterministically serialized bytes of the domain.
    /// This removes any platform-specific padding or compiler layout artifacts.
    fn canonicalize(&self) -> Vec<u8>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(BorshSerialize, BorshDeserialize)]
    struct ExampleGameState {
        pub player_x: i32,
        pub player_y: i32,
        pub active: bool,
    }

    impl DeterministicStateDomain for ExampleGameState {
        fn canonicalize(&self) -> Vec<u8> {
            // Borsh guarantees deterministic, padding-free little-endian serialization
            borsh::to_vec(self).expect("Failed to serialize ExampleGameState")
        }
    }

    #[test]
    fn test_canonical_serialization_is_deterministic() {
        let state1 = ExampleGameState {
            player_x: 42,
            player_y: -15,
            active: true,
        };

        let state2 = ExampleGameState {
            player_x: 42,
            player_y: -15,
            active: true,
        };

        // Identical semantic state must produce identical byte arrays.
        assert_eq!(state1.canonicalize(), state2.canonicalize());
    }

    #[test]
    fn test_canonical_serialization_numeric_handling() {
        let state = ExampleGameState {
            player_x: 0x0102_0304,
            player_y: 0,
            active: false,
        };
        
        let bytes = state.canonicalize();
        // borsh uses little-endian layout for numeric types
        assert_eq!(bytes[0], 0x04);
        assert_eq!(bytes[1], 0x03);
        assert_eq!(bytes[2], 0x02);
        assert_eq!(bytes[3], 0x01);
    }
}

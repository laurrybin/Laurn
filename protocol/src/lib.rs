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

pub mod codec;

use authority::{AuthorityCapability, AuthorityId};
use borsh::{BorshDeserialize, BorshSerialize};
use delta::StateDelta;
use epoch::EpochId;
use state::CanonicalState;
use transition::Transition;
use version_crate::ProtocolVersion;

/// A strictly bounded set of error codes for network communication.
/// Avoids the use of strings for deterministic and allocation-free error handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum ErrorCode {
    /// Generic catch-all or unknown error.
    Unknown,
    /// Protocol version mismatch between peers.
    IncompatibleVersion,
    /// Authority failed authentication.
    AuthenticationFailed,
    /// Sent message was malformed or failed validation.
    MalformedMessage,
    /// Transition or epoch is stale.
    StaleTransition,
    /// Action requires higher capability.
    InsufficientCapability,
    /// Generic verification failure.
    VerificationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct HandshakeRequest {
    pub client_version: ProtocolVersion,
    pub client_authority: AuthorityId,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct HandshakeResponse {
    pub server_version: ProtocolVersion,
    pub server_authority: AuthorityId,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct CapabilityNegotiation {
    pub requested_capabilities: AuthorityCapability,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct StateMessage {
    pub state: CanonicalState,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct TransitionMessage {
    pub transition: Transition,
    pub raw_payload: Vec<u8>,
    pub signature: [u8; 64],
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct DeltaMessage {
    pub epoch_id: EpochId,
    pub delta: StateDelta,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct VerificationMessage {
    pub transition_id: u64, // From transition.id.0
    pub is_valid: bool,
    pub error_code: Option<ErrorCode>,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ErrorMessage {
    pub code: ErrorCode,
}

#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct ReplayMetadata {
    pub session_id: [u8; 32],
    pub start_epoch: EpochId,
    pub end_epoch: Option<EpochId>,
    pub tick_rate: u32,
}

/// The exact internal payload of a LAURN network interaction.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum LaurnMessagePayload {
    HandshakeRequest(HandshakeRequest),
    HandshakeResponse(HandshakeResponse),
    CapabilityNegotiation(CapabilityNegotiation),
    State(StateMessage),
    Transition(TransitionMessage),
    Delta(DeltaMessage),
    Verification(VerificationMessage),
    Error(ErrorMessage),
    ReplayMetadata(ReplayMetadata),
}

/// The top-level envelope for all LAURN network messages.
/// By placing the `ProtocolVersion` at the outermost layer, receivers can
/// immediately reject incompatible messages before allocating memory or
/// attempting to parse the full semantic payload.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct LaurnMessage {
    pub version: ProtocolVersion,
    pub payload: LaurnMessagePayload,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_laurn_message_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let version = ProtocolVersion::new(1, 0, 0);
        let msg = LaurnMessage {
            version,
            payload: LaurnMessagePayload::Error(ErrorMessage {
                code: ErrorCode::IncompatibleVersion,
            }),
        };

        let bytes = borsh::to_vec(&msg)?;
        let decoded: LaurnMessage = borsh::from_slice(&bytes)?;

        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_version_extraction_before_full_decode() -> Result<(), Box<dyn std::error::Error>> {
        let version = ProtocolVersion::new(2, 5, 1);
        let msg = LaurnMessage {
            version,
            payload: LaurnMessagePayload::Error(ErrorMessage {
                code: ErrorCode::Unknown,
            }),
        };
        let bytes = borsh::to_vec(&msg)?;

        // The ProtocolVersion is the first thing in the struct.
        // It consists of three u32s (major, minor, patch), taking 12 bytes.
        let decoded_version: ProtocolVersion = borsh::from_slice(&bytes[0..12])?;

        assert_eq!(decoded_version.major, 2);
        assert_eq!(decoded_version.minor, 5);
        assert_eq!(decoded_version.patch, 1);
    }
}

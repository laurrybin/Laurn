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

use bitflags::bitflags;
use borsh::{BorshDeserialize, BorshSerialize};
use ed25519_dalek::{Signature, VerifyingKey};
use epoch::EpochId;

/// A unique identifier for a source of state transitions.
/// Backed by a 32-byte Ed25519 public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
#[repr(transparent)]
pub struct AuthorityId(pub [u8; 32]);

/// The assigned role of an Authority within the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
#[repr(u8)]
pub enum AuthorityRole {
    /// A connected player or agent.
    Client = 0,
    /// The canonical game server.
    Server = 1,
    /// An overarching arbiter for resolving disputes or controlling time.
    Arbiter = 2,
    /// A read-only observer.
    Spectator = 3,
}

bitflags! {
    /// Granular permissions assigned to an Authority.
    /// Uses a u32 bitmask for FFI-friendliness and fast bitwise checks.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(transparent)]
    pub struct AuthorityCapability: u32 {
        /// Can submit standard state transitions (e.g., input).
        const CAN_SUBMIT_TRANSITION = 1 << 0;
        /// Can spawn or destroy entities.
        const CAN_SPAWN             = 1 << 1;
        /// Can explicitly advance or mutate the epoch/time.
        const CAN_AUTHORIZE_TIME    = 1 << 2;
        /// Has absolute authority over the entire simulation (Admin).
        const CAN_ADMINISTER        = 1 << 31;
    }
}

impl BorshSerialize for AuthorityCapability {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.bits().serialize(writer)
    }
}

impl BorshDeserialize for AuthorityCapability {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bits = u32::deserialize_reader(reader)?;
        AuthorityCapability::from_bits(bits)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid bitflags"))
    }
}

/// Explicitly binds an Authority to a specific Epoch,
/// preventing authorities from operating outside their granted session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct SessionBinding {
    pub epoch_id: EpochId,
}

/// Represents an authenticated actor in the system.
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Authority {
    pub id: AuthorityId,
    pub role: AuthorityRole,
    pub capabilities: AuthorityCapability,
    pub session_binding: Option<SessionBinding>,
}

/// The Engine responsible for managing authorities, their capabilities,
/// and verifying their cryptographic signatures.
#[derive(Debug, Default)]
pub struct AuthorityEngine {
    authorities: std::collections::HashMap<[u8; 32], Authority>,
}

impl AuthorityEngine {
    #[must_use]
    pub fn new() -> Self {
        Self {
            authorities: std::collections::HashMap::new(),
        }
    }

    /// Registers a new authority in the engine.
    ///
    /// # Errors
    /// Returns an error if the authority already exists.
    pub fn register_authority(&mut self, authority: Authority) -> Result<(), &'static str> {
        if self.authorities.contains_key(&authority.id.0) {
            return Err("Authority already exists");
        }
        self.authorities.insert(authority.id.0, authority);
        Ok(())
    }

    /// Updates or rotates an existing authority (e.g., changing roles or capabilities).
    ///
    /// # Errors
    /// Returns an error if the authority does not exist.
    pub fn update_authority(&mut self, authority: Authority) -> Result<(), &'static str> {
        if !self.authorities.contains_key(&authority.id.0) {
            return Err("Authority not found");
        }
        self.authorities.insert(authority.id.0, authority);
        Ok(())
    }

    /// Removes an authority, immediately revoking all capabilities.
    ///
    /// # Errors
    /// Returns an error if the authority does not exist.
    pub fn remove_authority(&mut self, id: &AuthorityId) -> Result<(), &'static str> {
        if self.authorities.remove(&id.0).is_none() {
            return Err("Authority not found");
        }
        Ok(())
    }

    /// Checks if a given authority has a specific capability.
    /// Returns false if the authority doesn't exist or lacks the capability.
    #[must_use]
    pub fn check_capability(&self, id: &AuthorityId, capability: AuthorityCapability) -> bool {
        if let Some(auth) = self.authorities.get(&id.0) {
            return auth.capabilities.contains(capability);
        }
        false
    }

    /// Validates an Ed25519 signature over a payload using the authority's public key.
    ///
    /// # Errors
    /// Returns an error if the authority does not exist, if the public key is malformed,
    /// or if the signature is mathematically invalid.
    pub fn verify_signature(
        &self,
        id: &AuthorityId,
        payload: &[u8],
        signature_bytes: &[u8; 64],
    ) -> Result<(), &'static str> {
        // Ensure authority is known
        if !self.authorities.contains_key(&id.0) {
            return Err("Unauthorized authority");
        }

        // Construct verifying key from the 32-byte AuthorityId
        let verifying_key = VerifyingKey::from_bytes(&id.0).map_err(|_| "Malformed public key")?;

        // Construct signature
        let signature = Signature::from_bytes(signature_bytes);

        // Verify mathematically
        verifying_key
            .verify_strict(payload, &signature)
            .map_err(|_| "Invalid signature")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    fn generate_keypair() -> (SigningKey, AuthorityId) {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        (signing_key, AuthorityId(verifying_key.to_bytes()))
    }

    #[test]
    fn test_authority_capabilities() -> Result<(), Box<dyn std::error::Error>> {
        let mut engine = AuthorityEngine::new();
        let (_, auth_id) = generate_keypair();

        let authority = Authority {
            id: auth_id,
            role: AuthorityRole::Client,
            capabilities: AuthorityCapability::CAN_SUBMIT_TRANSITION,
            session_binding: None,
        };

        engine.register_authority(authority)?;

        assert!(engine.check_capability(&auth_id, AuthorityCapability::CAN_SUBMIT_TRANSITION));
        assert!(!engine.check_capability(&auth_id, AuthorityCapability::CAN_AUTHORIZE_TIME));

        // Unauthorized (unknown) authority checks gracefully fail
        let (_, unknown_id) = generate_keypair();
        assert!(!engine.check_capability(&unknown_id, AuthorityCapability::CAN_SUBMIT_TRANSITION));
        Ok(())
    }

    #[test]
    fn test_cryptographic_authentication() -> Result<(), Box<dyn std::error::Error>> {
        let mut engine = AuthorityEngine::new();
        let (signing_key, auth_id) = generate_keypair();

        let authority = Authority {
            id: auth_id,
            role: AuthorityRole::Server,
            capabilities: AuthorityCapability::CAN_ADMINISTER,
            session_binding: None,
        };

        engine.register_authority(authority)?;

        let payload = b"state_transition_payload_12345";
        let signature = signing_key.sign(payload);

        // Valid signature should pass
        assert!(engine
            .verify_signature(&auth_id, payload, &signature.to_bytes())
            .is_ok());

        // Tampered payload should fail
        let bad_payload = b"state_transition_payload_99999";
        assert!(engine
            .verify_signature(&auth_id, bad_payload, &signature.to_bytes())
            .is_err());

        // Unauthorized authority trying to verify should fail
        let (_, unknown_id) = generate_keypair();
        assert!(engine
            .verify_signature(&unknown_id, payload, &signature.to_bytes())
            .is_err());
        Ok(())
    }

    #[test]
    fn test_authority_rotation() -> Result<(), Box<dyn std::error::Error>> {
        let mut engine = AuthorityEngine::new();
        let (_, auth_id) = generate_keypair();

        let authority = Authority {
            id: auth_id,
            role: AuthorityRole::Client,
            capabilities: AuthorityCapability::CAN_SUBMIT_TRANSITION,
            session_binding: None,
        };

        engine.register_authority(authority.clone())?;
        assert!(engine.check_capability(&auth_id, AuthorityCapability::CAN_SUBMIT_TRANSITION));

        // Update capabilities
        let mut updated = authority;
        updated.capabilities =
            AuthorityCapability::CAN_SUBMIT_TRANSITION | AuthorityCapability::CAN_SPAWN;
        engine.update_authority(updated)?;
        assert!(engine.check_capability(&auth_id, AuthorityCapability::CAN_SPAWN));

        // Remove authority
        engine.remove_authority(&auth_id)?;
        assert!(!engine.check_capability(&auth_id, AuthorityCapability::CAN_SUBMIT_TRANSITION));
        Ok(())
    }
}

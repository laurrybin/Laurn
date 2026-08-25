use authority::AuthorityId;
use borsh::{BorshDeserialize, BorshSerialize};
use transition::TransitionCommitment;
use ed25519_dalek::{Signature, VerifyingKey, Verifier};
use epoch::EpochId;

pub mod platform;

/// Deterministic domain separator for evidence signature binding.
pub const EVIDENCE_DOMAIN_V1: &[u8] = b"LAURN_EVIDENCE_V1";

/// `EvidenceId` is a deterministic 32-byte identifier for an `ExecutionEvidence`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
pub struct EvidenceId(pub [u8; 32]);

/// Specifies the type of trusted environment that produced the evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum EvidenceType {
    /// A trusted backend server generated this evidence.
    ServerAuthoritative,
    /// An Intel SGX secure enclave generated this evidence.
    IntelSgx,
    /// An AWS Nitro Enclave generated this evidence.
    AwsNitro,
}

/// Represents cryptographic proof that a transition occurred in a trusted execution environment.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ExecutionEvidence {
    /// The unique identifier of this evidence.
    pub id: EvidenceId,
    /// The type of environment that produced this evidence.
    pub evidence_type: EvidenceType,
    /// The authority that generated the attestation.
    pub issuer: AuthorityId,
    /// The epoch during which this transition was evaluated.
    pub epoch_id: EpochId,
    /// The UTC timestamp in milliseconds when the evaluation occurred.
    pub timestamp_ms: u64,
    /// Cryptographic commitment to the exact state transition this evidence covers.
    pub transition_commitment: TransitionCommitment,
    /// The opaque bytes holding the real platform attestation (e.g., an SGX quote or JWT).
    pub raw_attestation: Vec<u8>,
    /// An Ed25519 signature from the issuer over the deterministic binding payload.
    pub signature: [u8; 64],
}

impl ExecutionEvidence {
    /// Computes the deterministic payload that the issuer must sign.
    #[must_use]
    pub fn signature_payload(&self) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(EVIDENCE_DOMAIN_V1);
        payload.extend_from_slice(&self.id.0);
        
        let type_byte = match self.evidence_type {
            EvidenceType::ServerAuthoritative => 0,
            EvidenceType::IntelSgx => 1,
            EvidenceType::AwsNitro => 2,
        };
        payload.push(type_byte);
        
        payload.extend_from_slice(&self.issuer.0);
        payload.extend_from_slice(&self.epoch_id.0);
        payload.extend_from_slice(&self.timestamp_ms.to_le_bytes());
        payload.extend_from_slice(self.transition_commitment.as_bytes());
        
        // Include the hash of the raw attestation to bind it without copying potentially large bytes
        let attestation_hash = blake3::hash(&self.raw_attestation);
        payload.extend_from_slice(attestation_hash.as_bytes());
        
        payload
    }

    /// Cryptographically checks the issuer signature over the bound data, proving the issuer
    /// endorsed this evidence for this specific transition.
    ///
    /// NOTE: This does NOT verify the underlying `raw_attestation` from the hardware platform.
    #[must_use]
    pub fn verify_signature(&self, public_key: &VerifyingKey) -> bool {
        let payload = self.signature_payload();
        let sig = Signature::from_bytes(&self.signature);
        public_key.verify(&payload, &sig).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{SigningKey, Signer};
    use rand::rngs::OsRng;

    #[test]
    fn test_evidence_verification() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        
        let issuer_bytes = verifying_key.to_bytes();
        let issuer = AuthorityId(issuer_bytes);

        let mut evidence = ExecutionEvidence {
            id: EvidenceId([1u8; 32]),
            evidence_type: EvidenceType::ServerAuthoritative,
            issuer,
            epoch_id: EpochId([10u8; 32]),
            timestamp_ms: 1234567890,
            transition_commitment: TransitionCommitment([2u8; 32]),
            raw_attestation: vec![0xca, 0xfe, 0xba, 0xbe],
            signature: [0u8; 64],
        };

        let payload = evidence.signature_payload();
        let signature = signing_key.sign(&payload);
        evidence.signature = signature.to_bytes();

        assert!(evidence.verify_signature(&verifying_key));
    }

    #[test]
    fn test_evidence_tampering() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        
        let issuer = AuthorityId(verifying_key.to_bytes());

        let mut evidence = ExecutionEvidence {
            id: EvidenceId([1u8; 32]),
            evidence_type: EvidenceType::IntelSgx,
            issuer,
            epoch_id: EpochId([10u8; 32]),
            timestamp_ms: 1234567890,
            transition_commitment: TransitionCommitment([2u8; 32]),
            raw_attestation: vec![0xca, 0xfe, 0xba, 0xbe],
            signature: [0u8; 64],
        };

        let payload = evidence.signature_payload();
        let signature = signing_key.sign(&payload);
        evidence.signature = signature.to_bytes();

        // Alter timestamp
        evidence.timestamp_ms = 1234567891;
        assert!(!evidence.verify_signature(&verifying_key));
    }
}

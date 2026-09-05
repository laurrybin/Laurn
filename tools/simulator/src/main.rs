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
use commitment::StateCommitment;
use ed25519_dalek::{Signer, SigningKey};
use epoch::EpochId;
use evidence::{EvidenceId, EvidenceType, ExecutionEvidence};
use protocol::{LaurnMessage, LaurnMessagePayload, TransitionMessage};
use rand::{rngs::StdRng, Rng, SeedableRng};
use replay::divergence::{DivergenceAnalyzer, DivergenceReason};
use replay::ReplayRecorder;
use transition::{Transition, TransitionCommitment, TransitionId, TransitionMetadata};
use version_crate::ProtocolVersion;

struct ServerNode {
    signing_key: SigningKey,
    authority_id: AuthorityId,
    state_commitment: StateCommitment,
    replay_recorder: ReplayRecorder,
}

impl ServerNode {
    fn new(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut secret = [0u8; 32];
        rng.fill(&mut secret);
        let signing_key = SigningKey::from_bytes(&secret);
        let verifying_key = signing_key.verifying_key();

        let authority_id = AuthorityId(verifying_key.to_bytes());
        let state_commitment = StateCommitment([0; 32]);
        let replay_recorder = ReplayRecorder::new(state_commitment);

        Self {
            signing_key,
            authority_id,
            state_commitment,
            replay_recorder,
        }
    }
}

fn main() -> Result<(), String> {
    println!("LAURN replay divergence simulator");

    let mut s1 = ServerNode::new(1);
    let mut s2 = ServerNode::new(2);
    let mut s3 = ServerNode::new(3);

    println!("Initialized 3 server nodes");

    run_epochs(&mut s1, &mut s2, &mut s3)
}

fn run_epochs(s1: &mut ServerNode, s2: &mut ServerNode, s3: &mut ServerNode) -> Result<(), String> {
    let num_epochs: u8 = 100;
    let desync_epoch: u8 = 75;

    for epoch_idx in 1..=num_epochs {
        let current_epoch = EpochId([epoch_idx; 32]);

        // Construct a deterministic payload
        let mut raw_payload = vec![0u8; 32];
        raw_payload[0] = epoch_idx;
        let payload_commitment = TransitionCommitment::compute(&raw_payload);

        let input_state = s1.state_commitment;

        // Compute the reference output state
        let mut next_hash = [0u8; 32];
        next_hash[0] = epoch_idx;
        next_hash[1] = 0xAA;
        let reference_output_state = StateCommitment(next_hash);

        // Server 2 computes it incorrectly on desync epoch
        let s2_output_state = if epoch_idx == desync_epoch {
            println!("Injecting desync into server 2 at epoch {desync_epoch}");
            let mut bad_hash = next_hash;
            bad_hash[1] = 0xBB; // Desync
            StateCommitment(bad_hash)
        } else {
            reference_output_state
        };

        let transition = Transition {
            id: TransitionId(u64::from(epoch_idx)),
            metadata: TransitionMetadata {
                authority_id: s1.authority_id,
                epoch_id: current_epoch,
                timestamp_ms: u64::from(epoch_idx) * 1000,
            },
            input_state,
            output_state: reference_output_state,
            payload_commitment,
        };

        // Create LaurnMessage
        let transition_class = 1u32;
        let transition_bytes =
            verification::transition_signing_bytes(&transition, transition_class)
                .map_err(|e| e.to_string())?;
        let signature = s1.signing_key.sign(&transition_bytes).to_bytes();

        let transition_msg = TransitionMessage {
            transition,
            transition_class,
            raw_payload,
            signature,
        };

        let laurn_msg = LaurnMessage {
            version: ProtocolVersion::current(),
            payload: LaurnMessagePayload::Transition(transition_msg),
        };

        let msg_bytes = borsh::to_vec(&laurn_msg).map_err(|e| e.to_string())?;

        // Add frames to recorders
        s1.replay_recorder
            .add_frame(msg_bytes.clone(), reference_output_state);
        s3.replay_recorder
            .add_frame(msg_bytes.clone(), reference_output_state);

        // Server 2 gets a corrupted output state expectation
        s2.replay_recorder
            .add_frame(msg_bytes.clone(), s2_output_state);

        s1.state_commitment = reference_output_state;
        s3.state_commitment = reference_output_state;
        s2.state_commitment = s2_output_state;

        if epoch_idx == desync_epoch {
            println!("Epoch {epoch_idx} Commitments:");
            println!("    Server 1: {:?}", s1.state_commitment.0);
            println!("    Server 2: {:?}", s2.state_commitment.0);
            println!("    Server 3: {:?}", s3.state_commitment.0);

            assert_ne!(
                s1.state_commitment, s2.state_commitment,
                "Desync failed! Server 1 and 2 match."
            );
            assert_eq!(
                s1.state_commitment, s3.state_commitment,
                "Reference servers should match."
            );
            println!("Injected commitment mismatch detected.");

            analyze_injected_divergence(
                s1,
                s2,
                current_epoch,
                epoch_idx,
                payload_commitment,
                reference_output_state,
                s2_output_state,
            )?;
            return Ok(());
        }
    }
    Ok(())
}

fn analyze_injected_divergence(
    s1: &ServerNode,
    s2: &ServerNode,
    current_epoch: EpochId,
    epoch_idx: u8,
    payload_commitment: TransitionCommitment,
    reference_output_state: StateCommitment,
    s2_output_state: StateCommitment,
) -> Result<(), String> {
    // Generate Evidence
    println!("Generating execution evidence from server 1...");

    let mut evidence = ExecutionEvidence {
        id: EvidenceId([1u8; 32]),
        evidence_type: EvidenceType::ServerAuthoritative,
        issuer: s1.authority_id,
        epoch_id: current_epoch,
        timestamp_ms: u64::from(epoch_idx) * 1000,
        transition_commitment: payload_commitment,
        raw_attestation: vec![0u8; 32],
        signature: [0u8; 64],
    };

    let evidence_payload = evidence.signature_payload();
    evidence.signature = s1.signing_key.sign(&evidence_payload).to_bytes();

    println!("Verifying evidence signature...");
    let verified = evidence.verify_signature(&s1.signing_key.verifying_key());

    assert!(verified, "Evidence verification failed!");
    println!("Evidence signature verified successfully.");

    // Analyze divergence
    println!("Analyzing replay divergence...");
    let s1_bytes = s1.replay_recorder.serialize().map_err(|e| e.to_string())?;
    let s2_bytes = s2.replay_recorder.serialize().map_err(|e| e.to_string())?;

    let mut auth_reader = replay::ReplayReader::new(&s1_bytes).map_err(|e| e.to_string())?;
    let mut test_reader = replay::ReplayReader::new(&s2_bytes).map_err(|e| e.to_string())?;

    let report = DivergenceAnalyzer::analyze(&mut auth_reader, &mut test_reader);
    match report {
        Some(r) => {
            println!(
                "Divergence Report: Frame {}, Reason: {:?}",
                r.frame_index, r.reason
            );
            if let DivergenceReason::CommitmentMismatch { expected, actual } = r.reason {
                assert_eq!(expected, reference_output_state);
                assert_eq!(actual, s2_output_state);
                println!("Replay divergence analysis detected the injected commitment mismatch.");
            } else {
                return Err("Unexpected divergence reason".to_string());
            }
        }
        None => return Err("Analyzer failed to find the divergence!".to_string()),
    }

    println!("Simulation complete: injected divergence detected and evidence signature verified.");
    Ok(())
}

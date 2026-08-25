use authority::AuthorityId;
use commitment::{CommitmentEngine, StateCommitment};
use transition::{Transition, TransitionCommitment, TransitionId, TransitionMetadata};
use epoch::EpochId;
use evidence::{ExecutionEvidence, EvidenceId, EvidenceType};
use protocol::{LaurnMessage, LaurnMessagePayload, TransitionMessage};
use version_crate::ProtocolVersion;
use replay::ReplayRecorder;
use replay::divergence::{DivergenceAnalyzer, DivergenceReason};
use rand::{Rng, rngs::StdRng, SeedableRng};
use ed25519_dalek::{SigningKey, Signer};
use borsh::BorshSerialize;

struct ServerNode {
    id: usize,
    signing_key: SigningKey,
    authority_id: AuthorityId,
    state_commitment: StateCommitment,
    replay_recorder: ReplayRecorder,
}

impl ServerNode {
    fn new(id: usize) -> Self {
        let mut rng = StdRng::seed_from_u64(id as u64);
        let mut secret = [0u8; 32];
        rng.fill(&mut secret);
        let signing_key = SigningKey::from_bytes(&secret);
        let verifying_key = signing_key.verifying_key();
        
        let authority_id = AuthorityId(verifying_key.to_bytes());
        let state_commitment = StateCommitment([0; 32]);
        let replay_recorder = ReplayRecorder::new(state_commitment);

        Self {
            id,
            signing_key,
            authority_id,
            state_commitment,
            replay_recorder,
        }
    }
}

fn main() {
    println!("============================================================");
    println!("LAURN END-TO-END SYSTEM VALIDATION SIMULATOR");
    println!("============================================================");

    let mut s1 = ServerNode::new(1);
    let mut s2 = ServerNode::new(2);
    let mut s3 = ServerNode::new(3);

    println!("[*] Initialized 3 Server Nodes");

    let num_epochs = 100;
    let desync_epoch = 75; // Inject desync at epoch 75

    for epoch_idx in 1..=num_epochs {
        let current_epoch = EpochId([epoch_idx as u8; 32]);
        
        // Construct the raw payload
        let raw_payload = format!("Move to X: {}", epoch_idx).into_bytes();
        let payload_commitment = TransitionCommitment::compute(&raw_payload);
        
        let input_state = s1.state_commitment;
        
        // Honest servers compute next state hash correctly
        let mut next_hash = [0u8; 32];
        next_hash[0] = epoch_idx as u8;
        next_hash[1] = 0xAA;
        let honest_output_state = StateCommitment(next_hash);

        // Server 2 computes it incorrectly on desync epoch
        let s2_output_state = if epoch_idx == desync_epoch {
            println!("\n[!] ===============================================");
            println!("[!] INJECTING DESYNC INTO SERVER 2 AT EPOCH {}", desync_epoch);
            println!("[!] ===============================================\n");
            let mut bad_hash = next_hash;
            bad_hash[1] = 0xBB; // Desync
            StateCommitment(bad_hash)
        } else {
            honest_output_state
        };

        let transition = Transition {
            id: TransitionId(epoch_idx as u64),
            metadata: TransitionMetadata {
                authority_id: s1.authority_id,
                epoch_id: current_epoch,
                timestamp_ms: epoch_idx as u64 * 1000,
            },
            input_state,
            output_state: honest_output_state,
            payload_commitment,
        };

        // Create LaurnMessage
        let payload = b"auth_signature_dummy";
        let signature = s1.signing_key.sign(payload).to_bytes();
        
        let transition_msg = TransitionMessage {
            transition,
            raw_payload,
            signature,
        };
        
        let laurn_msg = LaurnMessage {
            version: ProtocolVersion::current(),
            payload: LaurnMessagePayload::Transition(transition_msg),
        };
        
        let msg_bytes = borsh::to_vec(&laurn_msg).unwrap();

        // Add frames to recorders
        s1.replay_recorder.add_frame(msg_bytes.clone(), honest_output_state);
        s3.replay_recorder.add_frame(msg_bytes.clone(), honest_output_state);

        // Server 2 gets a corrupted output state expectation
        s2.replay_recorder.add_frame(msg_bytes.clone(), s2_output_state);

        s1.state_commitment = honest_output_state;
        s3.state_commitment = honest_output_state;
        s2.state_commitment = s2_output_state;

        if epoch_idx == desync_epoch {
            println!("[*] Epoch {} Commitments:", epoch_idx);
            println!("    Server 1: {:?}", s1.state_commitment.0);
            println!("    Server 2: {:?}", s2.state_commitment.0);
            println!("    Server 3: {:?}", s3.state_commitment.0);
            
            assert_ne!(s1.state_commitment, s2.state_commitment, "Desync failed! Server 1 and 2 match.");
            assert_eq!(s1.state_commitment, s3.state_commitment, "Honest servers should match!");
            println!("[*] Divergence correctly detected via Commitment mismatch.");

            // Generate Evidence
            println!("[*] Generating Execution Evidence from Server 1 (Honest)...");
            
            let mut evidence = ExecutionEvidence {
                id: EvidenceId([1u8; 32]),
                evidence_type: EvidenceType::ServerAuthoritative,
                issuer: s1.authority_id,
                epoch_id: current_epoch,
                timestamp_ms: 1234567890,
                transition_commitment: payload_commitment,
                raw_attestation: vec![0xca, 0xfe, 0xba, 0xbe],
                signature: [0u8; 64],
            };
            
            let evidence_payload = evidence.signature_payload();
            evidence.signature = s1.signing_key.sign(&evidence_payload).to_bytes();

            println!("[*] Verifying Execution Evidence...");
            let verified = evidence.verify_signature(&s1.signing_key.verifying_key());
            
            assert!(verified, "Evidence verification failed!");
            println!("[*] Evidence verified successfully.");
            
            // Analyze divergence
            println!("[*] Analyzing Divergence via Replay Buffers...");
            let s1_bytes = s1.replay_recorder.serialize().unwrap();
            let s2_bytes = s2.replay_recorder.serialize().unwrap();
            
            let mut auth_reader = replay::ReplayReader::new(&s1_bytes).unwrap();
            let mut test_reader = replay::ReplayReader::new(&s2_bytes).unwrap();
            
            let report = DivergenceAnalyzer::analyze(&mut auth_reader, &mut test_reader);
            match report {
                Some(r) => {
                    println!("[*] Divergence Report: Frame {}, Reason: {:?}", r.frame_index, r.reason);
                    if let DivergenceReason::CommitmentMismatch { expected, actual } = r.reason {
                        assert_eq!(expected, honest_output_state);
                        assert_eq!(actual, s2_output_state);
                        println!("[*] Math Soundness Verified: Simulator caught the desync successfully.");
                    } else {
                        panic!("Unexpected divergence reason");
                    }
                }
                None => panic!("Analyzer failed to find the divergence!"),
            }

            println!("\n============================================================");
            println!("SIMULATION COMPLETE: Mathematical soundness proven.");
            println!("============================================================");
            return;
        }
    }
}

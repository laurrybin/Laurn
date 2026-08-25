use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::rngs::OsRng;
use ed25519_dalek::{Signer, SigningKey};

use protocol::{LaurnMessage, LaurnMessagePayload, TransitionMessage};
use commitment::{StateCommitment, CommitmentEngine, TRANSITION_DOMAIN_V1};
use authority::{AuthorityId, AuthorityEngine, AuthorityCapability};
use epoch::{EpochId, EpochEngine};
use verification::{VerificationEngine, VerificationContext};
use policy::{PolicyEngine, Policy, TransitionClass};
use version::ProtocolVersion;
use transition::{Transition, TransitionId, TransitionMetadata, TransitionCommitment};

fn generate_dummy_transition(signing_key: &SigningKey) -> LaurnMessage {
    let mut input_state = [0u8; 32];
    input_state[0] = 42;
    
    let payload_bytes = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let payload_commitment = TransitionCommitment::compute(&payload_bytes);
    
    let transition = Transition {
        id: TransitionId(1),
        metadata: TransitionMetadata {
            authority_id: AuthorityId(signing_key.verifying_key().to_bytes()),
            epoch_id: EpochId([1; 32]),
            timestamp_ms: 1000,
        },
        input_state: StateCommitment(input_state),
        output_state: StateCommitment([0; 32]),
        payload_commitment,
    };
    
    let serialized_transition = borsh::to_vec(&transition).unwrap();
    let signature = signing_key.sign(&serialized_transition).to_bytes();
    
    LaurnMessage {
        version: ProtocolVersion::new(1, 0, 0),
        payload: LaurnMessagePayload::Transition(TransitionMessage {
            transition,
            raw_payload: payload_bytes,
            signature,
        }),
    }
}

fn bench_commitment_latency(c: &mut Criterion) {
    let mut payload = vec![0u8; 1024]; // 1 KB state chunk
    for i in 0..payload.len() {
        payload[i] = (i % 256) as u8;
    }
    
    c.bench_function("Commitment Latency (1KB State Chunk)", |b| {
        b.iter(|| {
            let result = CommitmentEngine::compute(TRANSITION_DOMAIN_V1, black_box(&payload));
            black_box(result)
        })
    });
}

fn bench_transition_latency(c: &mut Criterion) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    
    c.bench_function("Transition Generation & Signing", |b| {
        b.iter(|| {
            let msg = generate_dummy_transition(black_box(&signing_key));
            black_box(msg)
        })
    });
}

fn bench_encoding_latency(c: &mut Criterion) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let msg = generate_dummy_transition(&signing_key);
    
    c.bench_function("Borsh Encode (Transition)", |b| {
        b.iter(|| {
            let encoded = borsh::to_vec(&msg).unwrap();
            black_box(encoded)
        })
    });
    
    let encoded = borsh::to_vec(&msg).unwrap();
    c.bench_function("Borsh Decode (Transition)", |b| {
        b.iter(|| {
            let decoded: LaurnMessage = borsh::from_slice(black_box(&encoded)).unwrap();
            black_box(decoded)
        })
    });
}

fn bench_verification_latency(c: &mut Criterion) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let msg = generate_dummy_transition(&signing_key);
    let raw_payload = borsh::to_vec(&msg).unwrap();
    
    let mut authority_engine = AuthorityEngine::new();
    let authority_id = AuthorityId(signing_key.verifying_key().to_bytes());
    authority_engine.register_authority(authority::Authority {
        id: authority_id,
        role: authority::AuthorityRole::Client,
        capabilities: AuthorityCapability::empty(),
        session_binding: None,
    }).unwrap();

    let mut epoch_engine = EpochEngine::new();
    let epoch_id = EpochId([1; 32]);
    epoch_engine.register_epoch(epoch::Epoch {
        id: epoch_id,
        sequence: 1,
        start_time_ms: 0,
        expiration_time_ms: 2000,
        status: epoch::EpochStatus::Pending,
        initial_state: StateCommitment([0; 32]),
    }).unwrap();
    epoch_engine.activate_epoch(epoch_id).unwrap();

    let policy_engine = PolicyEngine::new();
    let policy = Policy {
        protocol_version: 1,
        max_state_freshness_ms: 0,
        require_evidence: false,
        allowed_transition_classes: TransitionClass::all(),
        minimum_capability: AuthorityCapability::empty(),
    };
    let verifier = VerificationEngine::new();
    let seen_transitions = verification::replay::ReplayBuffer::default();
    
    let LaurnMessagePayload::Transition(t) = msg.payload else { panic!() };
    
    // The dummy transition sets output_state = [0; 32], so we must pass exactly that for expected
    let expected_output_state = [0u8; 32];
    
    c.bench_function("Verification Latency (Crypto + Protocol)", |b| {
        b.iter(|| {
            let ctx = VerificationContext {
                transition: &t.transition,
                raw_payload: &t.raw_payload,
                signature: &t.signature,
                expected_input_state: t.transition.input_state,
                generated_output_state: StateCommitment(expected_output_state),
                authority_engine: &authority_engine,
                epoch_engine: &epoch_engine,
                policy_engine: &policy_engine,
                policy: &policy,
                seen_transitions: &seen_transitions,
                parent_state_timestamp_ms: 1000,
                has_evidence: false,
                transition_protocol_version: 1,
                transition_class: TransitionClass::from_bits_truncate(1),
            };
            
            let result = verifier.verify(black_box(&ctx));
            assert_eq!(result, verification::VerificationResult::Valid);
            black_box(result)
        })
    });
}

criterion_group!(benches, bench_commitment_latency, bench_transition_latency, bench_encoding_latency, bench_verification_latency);
criterion_main!(benches);

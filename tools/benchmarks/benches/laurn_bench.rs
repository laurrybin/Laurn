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

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;

use authority::{AuthorityCapability, AuthorityEngine, AuthorityId};
use commitment::{CommitmentEngine, StateCommitment, TRANSITION_DOMAIN_V1};
use epoch::{EpochEngine, EpochId};
use policy::{Policy, PolicyEngine, TransitionClass};
use protocol::{LaurnMessage, LaurnMessagePayload, TransitionMessage};
use transition::{Transition, TransitionCommitment, TransitionId, TransitionMetadata};
use verification::{VerificationContext, VerificationEngine};
use version::ProtocolVersion;

fn generate_benchmark_transition(signing_key: &SigningKey) -> LaurnMessage {
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

    let serialized_transition = borsh::to_vec(&transition).unwrap_or_else(|_| vec![]);
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
    for (byte, value) in payload.iter_mut().zip((0u8..=u8::MAX).cycle()) {
        *byte = value;
    }

    c.bench_function("Commitment Latency (1KB State Chunk)", |b| {
        b.iter(|| {
            let result = CommitmentEngine::compute(TRANSITION_DOMAIN_V1, black_box(&payload));
            black_box(result)
        });
    });
}

fn bench_transition_latency(c: &mut Criterion) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);

    c.bench_function("Transition Generation & Signing", |b| {
        b.iter(|| {
            let msg = generate_benchmark_transition(black_box(&signing_key));
            black_box(msg)
        });
    });
}

fn bench_encoding_latency(c: &mut Criterion) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let msg = generate_benchmark_transition(&signing_key);

    c.bench_function("Borsh Encode (Transition)", |b| {
        b.iter(|| {
            let encoded = borsh::to_vec(&msg).unwrap_or_else(|_| vec![]);
            black_box(encoded)
        });
    });

    let encoded = borsh::to_vec(&msg).unwrap_or_else(|_| vec![]);
    c.bench_function("Borsh Decode (Transition)", |b| {
        b.iter(|| {
            let decoded: LaurnMessage = match borsh::from_slice(black_box(&encoded)) {
                Ok(msg) => msg,
                Err(_) => return,
            };
            black_box(decoded);
        });
    });
}

fn bench_verification_latency(c: &mut Criterion) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let msg = generate_benchmark_transition(&signing_key);

    let mut authority_engine = AuthorityEngine::new();
    let authority_id = AuthorityId(signing_key.verifying_key().to_bytes());
    let _ = authority_engine.register_authority(authority::Authority {
        id: authority_id,
        role: authority::AuthorityRole::Client,
        capabilities: AuthorityCapability::empty(),
        session_binding: None,
    });

    let mut epoch_engine = EpochEngine::new();
    let epoch_id = EpochId([1; 32]);
    let _ = epoch_engine.register_epoch(epoch::Epoch {
        id: epoch_id,
        sequence: 1,
        start_time_ms: 0,
        expiration_time_ms: 2000,
        status: epoch::EpochStatus::Pending,
        initial_state: StateCommitment([0; 32]),
    });
    let _ = epoch_engine.activate_epoch(epoch_id);

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

    let LaurnMessagePayload::Transition(t) = msg.payload else {
        return;
    };

    // The benchmark transition sets output_state = [0; 32], so we must pass exactly that for expected
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
        });
    });
}

criterion_group!(
    benches,
    bench_commitment_latency,
    bench_transition_latency,
    bench_encoding_latency,
    bench_verification_latency
);
criterion_main!(benches);

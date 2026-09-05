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

#![allow(unsafe_code)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::panic::catch_unwind;
use std::ptr;
use std::slice;

use authority::AuthorityEngine;
use commitment::{CommitmentEngine, StateCommitment};
use ed25519_dalek::Signer;
use epoch::{Epoch, EpochEngine, EpochId, EpochStatus};
use policy::{Policy, PolicyEngine, TransitionClass};
use protocol::codec::LaurnCodec;
use protocol::LaurnMessage;
use replay::divergence::DivergenceReason;
use transition::Transition;
use verification::{VerificationContext, VerificationEngine, VerificationResult};

pub mod logging;

/// The standard error code returned by all LAURN C APIs.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaurnResult {
    /// Operation succeeded.
    Success = 0,
    /// A null pointer was passed for a required argument.
    NullPointer = 1,
    /// A panic occurred across the FFI boundary.
    Panic = 2,
    /// Memory allocation failed.
    OutOfMemory = 3,
    /// Buffer provided was too small.
    BufferTooSmall = 4,
    /// Decoding failed due to truncation, bad magic bytes, or malformed data.
    DecodeFailed = 5,
    /// Verification of the transition failed.
    VerificationFailed = 6,
    /// Invalid configuration was provided.
    InvalidConfig = 7,
    /// Encoding failed.
    EncodeFailed = 8,
    /// Verification failed because the transition is a duplicate.
    VerificationDuplicate = 9,
    /// Verification failed because the input state mismatched.
    VerificationStateMismatch = 10,
    /// End of stream reached.
    EndOfStream = 11,
    /// Divergence detected between replay streams.
    DivergenceDetected = 12,
}

// Opaque Handles

pub struct LaurnAuthorityEngineHandle {
    pub(crate) inner: AuthorityEngine,
}

pub struct LaurnEpochEngineHandle {
    pub(crate) inner: EpochEngine,
}

pub struct LaurnPolicyEngineHandle {
    pub(crate) inner: PolicyEngine,
}

pub struct LaurnVerificationEngineHandle {
    pub(crate) inner: VerificationEngine,
    pub(crate) seen_transitions: std::sync::Mutex<verification::replay::ReplayBuffer>,
}

pub struct LaurnTransitionHandle {
    pub(crate) inner: Transition,
    pub(crate) transition_class: u32,
}

pub struct LaurnMessageHandle {
    pub(crate) inner: LaurnMessage,
}

pub struct LaurnPolicyHandle {
    pub(crate) inner: Policy,
}

// Error Handling Helper

fn diagnostic_signing_key() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[0x42; 32])
}

fn catch_unwind_ffi<F>(f: F) -> LaurnResult
where
    F: FnOnce() -> LaurnResult + std::panic::UnwindSafe,
{
    match catch_unwind(f) {
        Ok(result) => result,
        Err(_) => LaurnResult::Panic,
    }
}

// Authority Engine Operations

#[no_mangle]
pub unsafe extern "C" fn laurn_authority_engine_create(
    out_handle: *mut *mut LaurnAuthorityEngineHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if out_handle.is_null() {
            return LaurnResult::NullPointer;
        }

        let engine = AuthorityEngine::new();
        let handle = Box::new(LaurnAuthorityEngineHandle { inner: engine });

        *out_handle = Box::into_raw(handle);

        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_authority_engine_destroy(
    handle: *mut LaurnAuthorityEngineHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if handle.is_null() {
            return LaurnResult::NullPointer;
        }

        let _ = Box::from_raw(handle);
        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_authority_engine_register_diagnostic_authority(
    handle: *mut LaurnAuthorityEngineHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if handle.is_null() {
            return LaurnResult::NullPointer;
        }

        let engine = &mut (*handle).inner;

        let signing_key = diagnostic_signing_key();

        // Convert Dalek verifying key to our AuthorityId (which is 32 bytes)
        let verifying_key = signing_key.verifying_key();
        let pub_key_bytes = verifying_key.to_bytes();

        let authority = authority::Authority {
            id: authority::AuthorityId(pub_key_bytes),
            role: authority::AuthorityRole::Client,
            capabilities: authority::AuthorityCapability::CAN_SUBMIT_TRANSITION
                | authority::AuthorityCapability::CAN_SPAWN,
            session_binding: None,
        };

        match engine.register_authority(authority) {
            Ok(()) => LaurnResult::Success,
            Err(_) => LaurnResult::InvalidConfig, // Or could be Success if already registered
        }
    })
}

// Epoch Engine Operations

#[no_mangle]
pub unsafe extern "C" fn laurn_epoch_engine_create(
    out_handle: *mut *mut LaurnEpochEngineHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if out_handle.is_null() {
            return LaurnResult::NullPointer;
        }

        let engine = EpochEngine::new();
        let handle = Box::new(LaurnEpochEngineHandle { inner: engine });

        *out_handle = Box::into_raw(handle);

        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_epoch_engine_destroy(
    handle: *mut LaurnEpochEngineHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if handle.is_null() {
            return LaurnResult::NullPointer;
        }

        let _ = Box::from_raw(handle);
        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_epoch_engine_register(
    handle: *mut LaurnEpochEngineHandle,
    epoch_id: *const [u8; 32],
    sequence: u64,
    start_time_ms: u64,
    expiration_time_ms: u64,
    initial_state: *const [u8; 32],
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if handle.is_null() || epoch_id.is_null() || initial_state.is_null() {
            return LaurnResult::NullPointer;
        }

        let engine = &mut (*handle).inner;
        let epoch = Epoch {
            id: EpochId(*epoch_id),
            sequence,
            start_time_ms,
            expiration_time_ms,
            status: EpochStatus::Pending,
            initial_state: StateCommitment(*initial_state),
        };

        match engine.register_epoch(epoch) {
            Ok(()) => LaurnResult::Success,
            Err(_) => LaurnResult::InvalidConfig,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_epoch_engine_activate(
    handle: *mut LaurnEpochEngineHandle,
    epoch_id: *const [u8; 32],
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if handle.is_null() || epoch_id.is_null() {
            return LaurnResult::NullPointer;
        }

        match (*handle).inner.activate_epoch(EpochId(*epoch_id)) {
            Ok(()) => LaurnResult::Success,
            Err(_) => LaurnResult::InvalidConfig,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_epoch_engine_close(
    handle: *mut LaurnEpochEngineHandle,
    epoch_id: *const [u8; 32],
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if handle.is_null() || epoch_id.is_null() {
            return LaurnResult::NullPointer;
        }

        match (*handle).inner.close_epoch(EpochId(*epoch_id)) {
            Ok(()) => LaurnResult::Success,
            Err(_) => LaurnResult::InvalidConfig,
        }
    })
}

// Policy Engine Operations

#[no_mangle]
pub unsafe extern "C" fn laurn_policy_engine_create(
    out_handle: *mut *mut LaurnPolicyEngineHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if out_handle.is_null() {
            return LaurnResult::NullPointer;
        }

        let engine = PolicyEngine::new();
        let handle = Box::new(LaurnPolicyEngineHandle { inner: engine });

        *out_handle = Box::into_raw(handle);

        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_policy_engine_destroy(
    handle: *mut LaurnPolicyEngineHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if handle.is_null() {
            return LaurnResult::NullPointer;
        }

        let _ = Box::from_raw(handle);
        LaurnResult::Success
    })
}

// Policy Operations

#[no_mangle]
pub unsafe extern "C" fn laurn_policy_create_default(
    out_policy: *mut *mut LaurnPolicyHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if out_policy.is_null() {
            return LaurnResult::NullPointer;
        }

        let policy = policy::Policy {
            protocol_version: 1,
            max_state_freshness_ms: 5000,
            require_evidence: false,
            allowed_transition_classes: policy::TransitionClass::all(),
            minimum_capability: authority::AuthorityCapability::empty(),
        };
        let handle = Box::new(LaurnPolicyHandle { inner: policy });
        *out_policy = Box::into_raw(handle);
        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_policy_destroy(handle: *mut LaurnPolicyHandle) -> LaurnResult {
    catch_unwind_ffi(|| {
        if handle.is_null() {
            return LaurnResult::NullPointer;
        }

        let _ = Box::from_raw(handle);
        LaurnResult::Success
    })
}

// Verification Engine Operations

#[no_mangle]
pub unsafe extern "C" fn laurn_verification_engine_create(
    out_handle: *mut *mut LaurnVerificationEngineHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if out_handle.is_null() {
            return LaurnResult::NullPointer;
        }

        let engine = VerificationEngine::new();
        let handle = Box::new(LaurnVerificationEngineHandle {
            inner: engine,
            seen_transitions: std::sync::Mutex::new(verification::replay::ReplayBuffer::default()),
        });

        *out_handle = Box::into_raw(handle);

        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_verification_engine_destroy(
    handle: *mut LaurnVerificationEngineHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if handle.is_null() {
            return LaurnResult::NullPointer;
        }

        let _ = Box::from_raw(handle);
        LaurnResult::Success
    })
}

// Protocol Decoding

#[no_mangle]
pub unsafe extern "C" fn laurn_protocol_decode_message(
    buffer: *const u8,
    buffer_len: usize,
    out_message: *mut *mut LaurnMessageHandle,
    out_bytes_consumed: *mut usize,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if buffer.is_null() || out_message.is_null() || out_bytes_consumed.is_null() {
            return LaurnResult::NullPointer;
        }

        let stream = slice::from_raw_parts(buffer, buffer_len);

        match LaurnCodec::decode(stream) {
            Ok((msg, consumed)) => {
                let handle = Box::new(LaurnMessageHandle { inner: msg });
                *out_message = Box::into_raw(handle);
                *out_bytes_consumed = consumed;
                LaurnResult::Success
            }
            Err(_) => LaurnResult::DecodeFailed,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_message_destroy(handle: *mut LaurnMessageHandle) -> LaurnResult {
    catch_unwind_ffi(|| {
        if handle.is_null() {
            return LaurnResult::NullPointer;
        }

        let _ = Box::from_raw(handle);
        LaurnResult::Success
    })
}

// Protocol Encoding

fn build_transition(
    transition_id: u64,
    authority_id: [u8; 32],
    epoch_id: [u8; 32],
    timestamp_ms: u64,
    input_state_commitment: [u8; 32],
    output_state_commitment: [u8; 32],
    raw_payload: &[u8],
) -> Transition {
    Transition {
        id: transition::TransitionId(transition_id),
        metadata: transition::TransitionMetadata {
            authority_id: authority::AuthorityId(authority_id),
            epoch_id: epoch::EpochId(epoch_id),
            timestamp_ms,
        },
        input_state: commitment::StateCommitment(input_state_commitment),
        output_state: commitment::StateCommitment(output_state_commitment),
        payload_commitment: transition::TransitionCommitment::compute(raw_payload),
    }
}

#[no_mangle]
pub unsafe extern "C" fn laurn_protocol_encode_transition_message(
    protocol_version: u32,
    transition_class: u32,
    transition_id: u64,
    authority_id: *const [u8; 32],
    epoch_id: *const [u8; 32],
    timestamp_ms: u64,
    input_state_commitment: *const [u8; 32],
    output_state_commitment: *const [u8; 32],
    raw_payload: *const u8,
    raw_payload_len: usize,
    signature: *const [u8; 64],
    out_buffer: *mut *mut u8,
    out_buffer_len: *mut usize,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if raw_payload.is_null()
            || signature.is_null()
            || out_buffer.is_null()
            || out_buffer_len.is_null()
            || authority_id.is_null()
            || epoch_id.is_null()
            || input_state_commitment.is_null()
            || output_state_commitment.is_null()
        {
            return LaurnResult::NullPointer;
        }

        let raw_payload_slice = slice::from_raw_parts(raw_payload, raw_payload_len);
        let sig = *signature;

        let trans = build_transition(
            transition_id,
            *authority_id,
            *epoch_id,
            timestamp_ms,
            *input_state_commitment,
            *output_state_commitment,
            raw_payload_slice,
        );

        let t_msg = protocol::TransitionMessage {
            transition: trans,
            transition_class,
            raw_payload: raw_payload_slice.to_vec(),
            signature: sig,
        };

        let msg = protocol::LaurnMessage {
            version: version_crate::ProtocolVersion::new(protocol_version, 0, 0),
            payload: protocol::LaurnMessagePayload::Transition(t_msg),
        };

        match LaurnCodec::encode(&msg) {
            Ok(bytes) => {
                let mut buf = bytes.into_boxed_slice();
                *out_buffer_len = buf.len();
                *out_buffer = buf.as_mut_ptr();
                let _ = Box::into_raw(buf); // leak it to pass to C++
                LaurnResult::Success
            }
            Err(_) => LaurnResult::EncodeFailed,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_protocol_free_bytes(
    buffer: *mut u8,
    buffer_len: usize,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if buffer.is_null() {
            return LaurnResult::NullPointer;
        }
        let slice_ptr = std::ptr::slice_from_raw_parts_mut(buffer, buffer_len);
        drop(Box::from_raw(slice_ptr));
        LaurnResult::Success
    })
}

// Message Field Extraction

#[no_mangle]
pub unsafe extern "C" fn laurn_message_get_transition(
    message: *const LaurnMessageHandle,
    out_transition: *mut *mut LaurnTransitionHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if message.is_null() || out_transition.is_null() {
            return LaurnResult::NullPointer;
        }

        let msg = &(*message).inner;
        if let protocol::LaurnMessagePayload::Transition(ref t) = msg.payload {
            let handle = Box::new(LaurnTransitionHandle {
                inner: t.transition.clone(),
                transition_class: t.transition_class,
            });
            *out_transition = Box::into_raw(handle);
            LaurnResult::Success
        } else {
            LaurnResult::DecodeFailed
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_transition_destroy(
    handle: *mut LaurnTransitionHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if handle.is_null() {
            return LaurnResult::NullPointer;
        }

        let _ = Box::from_raw(handle);
        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_message_get_signature(
    message: *const LaurnMessageHandle,
    out_signature: *mut [u8; 64],
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if message.is_null() || out_signature.is_null() {
            return LaurnResult::NullPointer;
        }

        let msg = &(*message).inner;
        if let protocol::LaurnMessagePayload::Transition(ref t) = msg.payload {
            ptr::copy_nonoverlapping(t.signature.as_ptr(), (*out_signature).as_mut_ptr(), 64);
            LaurnResult::Success
        } else {
            LaurnResult::DecodeFailed
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_message_get_raw_payload(
    message: *const LaurnMessageHandle,
    out_payload: *mut *const u8,
    out_payload_len: *mut usize,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if message.is_null() || out_payload.is_null() || out_payload_len.is_null() {
            return LaurnResult::NullPointer;
        }

        let msg = &(*message).inner;
        if let protocol::LaurnMessagePayload::Transition(ref t) = msg.payload {
            *out_payload = t.raw_payload.as_ptr();
            *out_payload_len = t.raw_payload.len();
            LaurnResult::Success
        } else {
            LaurnResult::DecodeFailed
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_message_get_protocol_version(
    message: *const LaurnMessageHandle,
    out_version: *mut u32,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if message.is_null() || out_version.is_null() {
            return LaurnResult::NullPointer;
        }

        let msg = &(*message).inner;
        *out_version = msg.version.major; // Projecting semantic version to standard protocol integer format
        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_transition_get_class(
    transition: *const LaurnTransitionHandle,
    out_class: *mut u32,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if transition.is_null() || out_class.is_null() {
            return LaurnResult::NullPointer;
        }

        *out_class = (*transition).transition_class;
        LaurnResult::Success
    })
}

// State Commitment

#[no_mangle]
pub unsafe extern "C" fn laurn_transition_get_timestamp_ms(
    transition: *const LaurnTransitionHandle,
    out_timestamp_ms: *mut u64,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if transition.is_null() || out_timestamp_ms.is_null() {
            return LaurnResult::NullPointer;
        }

        *out_timestamp_ms = (*transition).inner.metadata.timestamp_ms;
        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_state_commitment_compute(
    buffer: *const u8,
    buffer_len: usize,
    out_hash: *mut [u8; 32],
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if buffer.is_null() || out_hash.is_null() {
            return LaurnResult::NullPointer;
        }

        let stream = slice::from_raw_parts(buffer, buffer_len);
        let commitment = CommitmentEngine::commit_state(stream);

        ptr::copy_nonoverlapping(commitment.0.as_ptr(), (*out_hash).as_mut_ptr(), 32);

        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_free_bytes(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }

    let slice_ptr = std::ptr::slice_from_raw_parts_mut(ptr, len);
    drop(Box::from_raw(slice_ptr));
}

// Verification

#[repr(C)]
pub struct LaurnVerificationParams {
    pub transition: *const LaurnTransitionHandle,
    pub raw_payload: *const u8,
    pub raw_payload_len: usize,
    pub signature: *const [u8; 64],
    pub expected_input_state: *const [u8; 32],
    pub generated_output_state: *const [u8; 32],
    pub authority_engine: *const LaurnAuthorityEngineHandle,
    pub epoch_engine: *const LaurnEpochEngineHandle,
    pub policy_engine: *const LaurnPolicyEngineHandle,
    pub policy: *const LaurnPolicyHandle,
    pub parent_state_timestamp_ms: u64,
    pub has_evidence: bool,
    pub transition_protocol_version: u32,
    pub transition_class: u32,
}

#[no_mangle]
pub unsafe extern "C" fn laurn_verify_transition(
    verifier: *const LaurnVerificationEngineHandle,
    params: *const LaurnVerificationParams,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if verifier.is_null() || params.is_null() {
            return LaurnResult::NullPointer;
        }

        let p = &*params;
        if p.transition.is_null()
            || p.raw_payload.is_null()
            || p.signature.is_null()
            || p.expected_input_state.is_null()
            || p.generated_output_state.is_null()
            || p.authority_engine.is_null()
            || p.epoch_engine.is_null()
            || p.policy_engine.is_null()
            || p.policy.is_null()
        {
            return LaurnResult::NullPointer;
        }

        let transition = &(*p.transition).inner;
        let raw_payload = slice::from_raw_parts(p.raw_payload, p.raw_payload_len);
        let signature = &*p.signature;
        let expected_input_state = StateCommitment(*p.expected_input_state);
        let generated_output_state = StateCommitment(*p.generated_output_state);
        let authority_engine = &(*p.authority_engine).inner;
        let epoch_engine = &(*p.epoch_engine).inner;
        let policy_engine = &(*p.policy_engine).inner;
        let policy = &(*p.policy).inner;
        let Some(transition_class) = TransitionClass::from_bits(p.transition_class) else {
            return LaurnResult::VerificationFailed;
        };

        let verifier = &*verifier;
        let Ok(mut seen_transitions) = verifier.seen_transitions.lock() else {
            return LaurnResult::Panic;
        };

        let result = {
            let ctx = VerificationContext {
                transition,
                raw_payload,
                signature,
                expected_input_state,
                generated_output_state,
                authority_engine,
                epoch_engine,
                policy_engine,
                policy,
                seen_transitions: &seen_transitions,
                parent_state_timestamp_ms: p.parent_state_timestamp_ms,
                has_evidence: p.has_evidence,
                transition_protocol_version: p.transition_protocol_version,
                transition_class,
            };

            verifier.inner.verify(&ctx)
        };

        match result {
            VerificationResult::Valid => {
                seen_transitions.insert(transition.id);
                LaurnResult::Success
            }
            VerificationResult::Duplicate(_) => LaurnResult::VerificationDuplicate,
            VerificationResult::StateMismatch(_) => LaurnResult::VerificationStateMismatch,
            _ => LaurnResult::VerificationFailed,
        }
    })
}

// Client Utilities (for Unreal Client to sign payloads)

#[no_mangle]
pub unsafe extern "C" fn laurn_diagnostic_sign_transition(
    transition_id: u64,
    transition_class: u32,
    epoch_id: *const [u8; 32],
    timestamp_ms: u64,
    input_state_commitment: *const [u8; 32],
    output_state_commitment: *const [u8; 32],
    raw_payload: *const u8,
    raw_payload_len: usize,
    out_authority_id: *mut [u8; 32],
    out_signature: *mut [u8; 64],
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if epoch_id.is_null()
            || input_state_commitment.is_null()
            || output_state_commitment.is_null()
            || raw_payload.is_null()
            || out_authority_id.is_null()
            || out_signature.is_null()
        {
            return LaurnResult::NullPointer;
        }

        let payload = slice::from_raw_parts(raw_payload, raw_payload_len);
        let signing_key = diagnostic_signing_key();
        let authority_id = signing_key.verifying_key().to_bytes();

        let transition = build_transition(
            transition_id,
            authority_id,
            *epoch_id,
            timestamp_ms,
            *input_state_commitment,
            *output_state_commitment,
            payload,
        );

        if TransitionClass::from_bits(transition_class).is_none() {
            return LaurnResult::InvalidConfig;
        }

        let Ok(signed_bytes) =
            verification::transition_signing_bytes(&transition, transition_class)
        else {
            return LaurnResult::EncodeFailed;
        };

        let signature = signing_key.sign(&signed_bytes);

        *out_authority_id = authority_id;
        ptr::copy_nonoverlapping(
            signature.to_bytes().as_ptr(),
            (*out_signature).as_mut_ptr(),
            64,
        );

        LaurnResult::Success
    })
}

// Replay Engine

pub struct LaurnReplayRecorderHandle {
    inner: replay::ReplayRecorder,
}

pub struct LaurnReplayReaderHandle {
    inner: replay::ReplayReader<'static>,
}

#[no_mangle]
pub unsafe extern "C" fn laurn_replay_recorder_create(
    initial_state: *const [u8; 32],
    out_handle: *mut *mut LaurnReplayRecorderHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if initial_state.is_null() || out_handle.is_null() {
            return LaurnResult::NullPointer;
        }
        let init_state = commitment::StateCommitment(*initial_state);
        let recorder = replay::ReplayRecorder::new(init_state);
        let handle = Box::new(LaurnReplayRecorderHandle { inner: recorder });
        *out_handle = Box::into_raw(handle);
        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_replay_recorder_add_frame(
    recorder: *mut LaurnReplayRecorderHandle,
    raw_payload: *const u8,
    raw_payload_len: usize,
    expected_output_state: *const [u8; 32],
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if recorder.is_null() || raw_payload.is_null() || expected_output_state.is_null() {
            return LaurnResult::NullPointer;
        }
        let payload = slice::from_raw_parts(raw_payload, raw_payload_len).to_vec();
        let out_state = commitment::StateCommitment(*expected_output_state);
        (*recorder).inner.add_frame(payload, out_state);
        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_replay_recorder_serialize(
    recorder: *const LaurnReplayRecorderHandle,
    out_bytes: *mut *mut u8,
    out_len: *mut usize,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if recorder.is_null() || out_bytes.is_null() || out_len.is_null() {
            return LaurnResult::NullPointer;
        }
        match (*recorder).inner.serialize() {
            Ok(bytes) => {
                let mut buffer = bytes.into_boxed_slice();
                *out_len = buffer.len();
                *out_bytes = buffer.as_mut_ptr();
                let _ = Box::into_raw(buffer);
                LaurnResult::Success
            }
            Err(_) => LaurnResult::DecodeFailed,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_replay_recorder_destroy(
    handle: *mut LaurnReplayRecorderHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if handle.is_null() {
            return LaurnResult::NullPointer;
        }
        drop(Box::from_raw(handle));
        LaurnResult::Success
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_replay_reader_create(
    buffer: *const u8,
    buffer_len: usize,
    out_handle: *mut *mut LaurnReplayReaderHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if buffer.is_null() || out_handle.is_null() {
            return LaurnResult::NullPointer;
        }
        let static_slice: &'static [u8] = slice::from_raw_parts(buffer, buffer_len);
        match replay::ReplayReader::new(static_slice) {
            Ok(reader) => {
                let handle = Box::new(LaurnReplayReaderHandle { inner: reader });
                *out_handle = Box::into_raw(handle);
                LaurnResult::Success
            }
            Err(_) => LaurnResult::DecodeFailed,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_replay_reader_next_frame(
    reader: *mut LaurnReplayReaderHandle,
    out_payload: *mut *mut u8,
    out_payload_len: *mut usize,
    out_expected_output_state: *mut [u8; 32],
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if reader.is_null()
            || out_payload.is_null()
            || out_payload_len.is_null()
            || out_expected_output_state.is_null()
        {
            return LaurnResult::NullPointer;
        }
        match (*reader).inner.next_frame() {
            Ok(Some(frame)) => {
                let mut buffer = frame.raw_payload.into_boxed_slice();
                *out_payload_len = buffer.len();
                *out_payload = buffer.as_mut_ptr();
                let _ = Box::into_raw(buffer);
                ptr::copy_nonoverlapping(
                    frame.expected_output_state.0.as_ptr(),
                    (*out_expected_output_state).as_mut_ptr(),
                    32,
                );
                LaurnResult::Success
            }
            Ok(None) => LaurnResult::EndOfStream,
            Err(_) => LaurnResult::DecodeFailed,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn laurn_replay_reader_destroy(
    handle: *mut LaurnReplayReaderHandle,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if handle.is_null() {
            return LaurnResult::NullPointer;
        }
        drop(Box::from_raw(handle));
        LaurnResult::Success
    })
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaurnDivergenceReason {
    ParentMismatch = 0,
    CommitmentMismatch = 1,
    EpochMismatch = 2,
    AuthorityMismatch = 3,
    PayloadMismatch = 4,
    LengthMismatch = 5,
    DecodeFailed = 6,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LaurnDivergenceReport {
    pub frame_index: u32,
    pub reason: LaurnDivergenceReason,
    pub expected_commitment: [u8; 32],
    pub actual_commitment: [u8; 32],
    pub expected_epoch: [u8; 32],
    pub actual_epoch: [u8; 32],
    pub expected_authority: [u8; 32],
    pub actual_authority: [u8; 32],
    pub expected_frames: u32,
    pub actual_frames: u32,
}

#[no_mangle]
pub unsafe extern "C" fn laurn_replay_analyze_divergence(
    auth_reader: *mut LaurnReplayReaderHandle,
    test_reader: *mut LaurnReplayReaderHandle,
    out_report: *mut LaurnDivergenceReport,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if auth_reader.is_null() || test_reader.is_null() || out_report.is_null() {
            return LaurnResult::NullPointer;
        }

        match replay::divergence::DivergenceAnalyzer::analyze(
            &mut (*auth_reader).inner,
            &mut (*test_reader).inner,
        ) {
            Some(report) => {
                let mut c_report = LaurnDivergenceReport {
                    frame_index: report.frame_index,
                    reason: LaurnDivergenceReason::DecodeFailed,
                    expected_commitment: [0; 32],
                    actual_commitment: [0; 32],
                    expected_epoch: [0; 32],
                    actual_epoch: [0; 32],
                    expected_authority: [0; 32],
                    actual_authority: [0; 32],
                    expected_frames: 0,
                    actual_frames: 0,
                };

                match report.reason {
                    DivergenceReason::ParentMismatch { expected, actual } => {
                        c_report.reason = LaurnDivergenceReason::ParentMismatch;
                        c_report.expected_commitment = expected.0;
                        c_report.actual_commitment = actual.0;
                    }
                    DivergenceReason::CommitmentMismatch { expected, actual } => {
                        c_report.reason = LaurnDivergenceReason::CommitmentMismatch;
                        c_report.expected_commitment = expected.0;
                        c_report.actual_commitment = actual.0;
                    }
                    DivergenceReason::EpochMismatch { expected, actual } => {
                        c_report.reason = LaurnDivergenceReason::EpochMismatch;
                        c_report.expected_epoch = expected.0;
                        c_report.actual_epoch = actual.0;
                    }
                    DivergenceReason::AuthorityMismatch { expected, actual } => {
                        c_report.reason = LaurnDivergenceReason::AuthorityMismatch;
                        c_report.expected_authority = expected.0;
                        c_report.actual_authority = actual.0;
                    }
                    DivergenceReason::PayloadMismatch => {
                        c_report.reason = LaurnDivergenceReason::PayloadMismatch;
                    }
                    DivergenceReason::LengthMismatch {
                        expected_frames,
                        actual_frames,
                    } => {
                        c_report.reason = LaurnDivergenceReason::LengthMismatch;
                        c_report.expected_frames = expected_frames;
                        c_report.actual_frames = actual_frames;
                    }
                    DivergenceReason::DecodeFailed => {
                        c_report.reason = LaurnDivergenceReason::DecodeFailed;
                    }
                }

                *out_report = c_report;
                LaurnResult::DivergenceDetected
            }
            None => LaurnResult::Success,
        }
    })
}

/// Returns the protocol version as an integer.
#[no_mangle]
pub extern "C" fn laurn_get_version() -> u32 {
    let version = version_crate::ProtocolVersion::current();
    (version.major << 16) | (version.minor << 8) | version.patch
}

/// Populates a buffer with the build info string (null-terminated).
#[no_mangle]
pub extern "C" fn laurn_get_build_info(
    buffer: *mut std::os::raw::c_char,
    buffer_len: usize,
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if buffer.is_null() {
            return LaurnResult::NullPointer;
        }

        let build_info = format!("LAURN Core v{}", version_crate::ProtocolVersion::current());
        let Ok(c_str) = std::ffi::CString::new(build_info) else {
            return LaurnResult::EncodeFailed;
        };
        let bytes = c_str.as_bytes_with_nul();

        if bytes.len() > buffer_len {
            return LaurnResult::BufferTooSmall;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer.cast::<u8>(), bytes.len());
        }

        LaurnResult::Success
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct VerificationTestHandles {
        authority_engine: *mut LaurnAuthorityEngineHandle,
        epoch_engine: *mut LaurnEpochEngineHandle,
        policy_engine: *mut LaurnPolicyEngineHandle,
        verifier: *mut LaurnVerificationEngineHandle,
        policy: *mut LaurnPolicyHandle,
    }

    impl VerificationTestHandles {
        fn create(epoch_id: &[u8; 32], input_state: &[u8; 32]) -> Self {
            unsafe {
                let mut handles = Self {
                    authority_engine: ptr::null_mut(),
                    epoch_engine: ptr::null_mut(),
                    policy_engine: ptr::null_mut(),
                    verifier: ptr::null_mut(),
                    policy: ptr::null_mut(),
                };

                assert_eq!(
                    laurn_authority_engine_create(std::ptr::from_mut(
                        &mut handles.authority_engine
                    )),
                    LaurnResult::Success
                );
                assert_eq!(
                    laurn_authority_engine_register_diagnostic_authority(handles.authority_engine),
                    LaurnResult::Success
                );
                assert_eq!(
                    laurn_epoch_engine_create(std::ptr::from_mut(&mut handles.epoch_engine)),
                    LaurnResult::Success
                );
                assert_eq!(
                    laurn_epoch_engine_register(
                        handles.epoch_engine,
                        std::ptr::from_ref(epoch_id),
                        1,
                        500,
                        1_500,
                        std::ptr::from_ref(input_state),
                    ),
                    LaurnResult::Success
                );
                assert_eq!(
                    laurn_epoch_engine_activate(handles.epoch_engine, std::ptr::from_ref(epoch_id)),
                    LaurnResult::Success
                );
                assert_eq!(
                    laurn_policy_engine_create(std::ptr::from_mut(&mut handles.policy_engine)),
                    LaurnResult::Success
                );
                assert_eq!(
                    laurn_policy_create_default(std::ptr::from_mut(&mut handles.policy)),
                    LaurnResult::Success
                );
                assert_eq!(
                    laurn_verification_engine_create(std::ptr::from_mut(&mut handles.verifier)),
                    LaurnResult::Success
                );

                handles
            }
        }

        fn destroy(self) {
            unsafe {
                assert_eq!(laurn_policy_destroy(self.policy), LaurnResult::Success);
                assert_eq!(
                    laurn_verification_engine_destroy(self.verifier),
                    LaurnResult::Success
                );
                assert_eq!(
                    laurn_policy_engine_destroy(self.policy_engine),
                    LaurnResult::Success
                );
                assert_eq!(
                    laurn_epoch_engine_destroy(self.epoch_engine),
                    LaurnResult::Success
                );
                assert_eq!(
                    laurn_authority_engine_destroy(self.authority_engine),
                    LaurnResult::Success
                );
            }
        }
    }

    #[test]
    fn test_ffi_engine_lifecycle() {
        unsafe {
            let mut handle: *mut LaurnAuthorityEngineHandle = ptr::null_mut();

            assert_eq!(
                laurn_authority_engine_create(std::ptr::from_mut(&mut handle)),
                LaurnResult::Success
            );
            assert!(!handle.is_null());

            assert_eq!(laurn_authority_engine_destroy(handle), LaurnResult::Success);
        }
    }

    #[test]
    fn test_ffi_epoch_lifecycle() {
        unsafe {
            let mut handle: *mut LaurnEpochEngineHandle = ptr::null_mut();
            let epoch_id = [7u8; 32];
            let initial_state = [0u8; 32];

            assert_eq!(
                laurn_epoch_engine_create(std::ptr::from_mut(&mut handle)),
                LaurnResult::Success
            );
            assert!(!handle.is_null());

            assert_eq!(
                laurn_epoch_engine_register(
                    handle,
                    std::ptr::from_ref(&epoch_id),
                    7,
                    1_000,
                    2_000,
                    std::ptr::from_ref(&initial_state),
                ),
                LaurnResult::Success
            );

            assert_eq!(
                laurn_epoch_engine_activate(handle, std::ptr::from_ref(&epoch_id)),
                LaurnResult::Success
            );

            assert!((*handle)
                .inner
                .validate_transition_binding(&EpochId(epoch_id), 1_500));

            assert_eq!(
                laurn_epoch_engine_close(handle, std::ptr::from_ref(&epoch_id)),
                LaurnResult::Success
            );

            assert!(!(*handle)
                .inner
                .validate_transition_binding(&EpochId(epoch_id), 1_500));

            assert_eq!(
                laurn_epoch_engine_activate(handle, std::ptr::from_ref(&epoch_id)),
                LaurnResult::InvalidConfig
            );

            assert_eq!(laurn_epoch_engine_destroy(handle), LaurnResult::Success);
        }
    }

    #[test]
    fn test_ffi_diagnostic_signature_verifies_and_replay_is_rejected() {
        unsafe {
            let raw_payload = [1u8; 32];
            let epoch_id = [1u8; 32];
            let input_state = [0u8; 32];
            let output_state = [0u8; 32];
            let transition_id = 1u64;
            let timestamp_ms = 1_000u64;
            let mut authority_id = [0u8; 32];
            let mut signature = [0u8; 64];

            let handles = VerificationTestHandles::create(&epoch_id, &input_state);
            let authority_engine = handles.authority_engine;
            let epoch_engine = handles.epoch_engine;
            let policy_engine = handles.policy_engine;
            let verifier = handles.verifier;
            let policy = handles.policy;

            assert_eq!(
                laurn_diagnostic_sign_transition(
                    transition_id,
                    TransitionClass::INPUT.bits(),
                    std::ptr::from_ref(&epoch_id),
                    timestamp_ms,
                    std::ptr::from_ref(&input_state),
                    std::ptr::from_ref(&output_state),
                    raw_payload.as_ptr(),
                    raw_payload.len(),
                    std::ptr::from_mut(&mut authority_id),
                    std::ptr::from_mut(&mut signature),
                ),
                LaurnResult::Success
            );

            let transition = LaurnTransitionHandle {
                inner: build_transition(
                    transition_id,
                    authority_id,
                    epoch_id,
                    timestamp_ms,
                    input_state,
                    output_state,
                    &raw_payload,
                ),
                transition_class: TransitionClass::INPUT.bits(),
            };

            let mut extracted_timestamp = 0u64;
            assert_eq!(
                laurn_transition_get_timestamp_ms(
                    std::ptr::from_ref(&transition),
                    std::ptr::from_mut(&mut extracted_timestamp),
                ),
                LaurnResult::Success
            );
            assert_eq!(extracted_timestamp, timestamp_ms);

            let params = LaurnVerificationParams {
                transition: std::ptr::from_ref(&transition),
                raw_payload: raw_payload.as_ptr(),
                raw_payload_len: raw_payload.len(),
                signature: std::ptr::from_ref(&signature),
                expected_input_state: std::ptr::from_ref(&input_state),
                generated_output_state: std::ptr::from_ref(&output_state),
                authority_engine,
                epoch_engine,
                policy_engine,
                policy,
                parent_state_timestamp_ms: timestamp_ms,
                has_evidence: false,
                transition_protocol_version: 1,
                transition_class: 1,
            };

            assert_eq!(
                laurn_verify_transition(verifier, std::ptr::from_ref(&params)),
                LaurnResult::Success
            );
            assert_eq!(
                laurn_verify_transition(verifier, std::ptr::from_ref(&params)),
                LaurnResult::VerificationDuplicate
            );

            handles.destroy();
        }
    }

    #[test]
    fn test_ffi_null_pointer_handling() {
        unsafe {
            assert_eq!(
                laurn_authority_engine_create(ptr::null_mut()),
                LaurnResult::NullPointer
            );
            assert_eq!(
                laurn_authority_engine_destroy(ptr::null_mut()),
                LaurnResult::NullPointer
            );
        }
    }
}

// Telemetry and Versioning

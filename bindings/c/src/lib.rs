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

#![allow(unsafe_code)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::panic::catch_unwind;
use std::ptr;
use std::slice;

use authority::AuthorityEngine;
use commitment::{CommitmentEngine, StateCommitment};
use delta::StateDelta;
use epoch::EpochEngine;
use policy::{Policy, PolicyEngine, TransitionClass};
use protocol::codec::LaurnCodec;
use protocol::LaurnMessage;
use state::CanonicalState;
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

// ----------------------------------------------------------------------------
// Opaque Handles
// ----------------------------------------------------------------------------

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
}

pub struct LaurnDeltaHandle {
    pub(crate) inner: StateDelta,
}

pub struct LaurnMessageHandle {
    pub(crate) inner: LaurnMessage,
}

pub struct LaurnCanonicalStateHandle {
    pub(crate) inner: CanonicalState,
}

pub struct LaurnPolicyHandle {
    pub(crate) inner: Policy,
}

// ----------------------------------------------------------------------------
// Error Handling Helper
// ----------------------------------------------------------------------------

fn catch_unwind_ffi<F>(f: F) -> LaurnResult
where
    F: FnOnce() -> LaurnResult + std::panic::UnwindSafe,
{
    match catch_unwind(f) {
        Ok(result) => result,
        Err(_) => LaurnResult::Panic,
    }
}

// ----------------------------------------------------------------------------
// Authority Engine Operations
// ----------------------------------------------------------------------------

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

        let secret_bytes = [0x42; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);

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
            Ok(_) => LaurnResult::Success,
            Err(_) => LaurnResult::InvalidConfig, // Or could be Success if already registered
        }
    })
}

// ----------------------------------------------------------------------------
// Epoch Engine Operations
// ----------------------------------------------------------------------------

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

// ----------------------------------------------------------------------------
// Policy Engine Operations
// ----------------------------------------------------------------------------

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

// ----------------------------------------------------------------------------
// Policy Operations
// ----------------------------------------------------------------------------

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

// ----------------------------------------------------------------------------
// Verification Engine Operations
// ----------------------------------------------------------------------------

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

// ----------------------------------------------------------------------------
// Protocol Decoding
// ----------------------------------------------------------------------------

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

// ----------------------------------------------------------------------------
// Protocol Encoding
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn laurn_protocol_encode_transition_message(
    protocol_version: u32,
    _transition_class: u32,
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

        let trans = transition::Transition {
            id: transition::TransitionId(transition_id),
            metadata: transition::TransitionMetadata {
                authority_id: authority::AuthorityId(*authority_id),
                epoch_id: epoch::EpochId(*epoch_id),
                timestamp_ms,
            },
            input_state: commitment::StateCommitment(*input_state_commitment),
            output_state: commitment::StateCommitment(*output_state_commitment),
            payload_commitment: transition::TransitionCommitment::compute(raw_payload_slice),
        };

        let t_msg = protocol::TransitionMessage {
            transition: trans,
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
        let _ = Box::from_raw(slice::from_raw_parts_mut(buffer, buffer_len));
        LaurnResult::Success
    })
}

// ----------------------------------------------------------------------------
// Message Field Extraction
// ----------------------------------------------------------------------------

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

        // Default to transition class 1 if transition doesn't explicitly declare it
        *out_class = 1;
        LaurnResult::Success
    })
}

// ----------------------------------------------------------------------------
// State Commitment
// ----------------------------------------------------------------------------

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
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

// ----------------------------------------------------------------------------
// Verification
// ----------------------------------------------------------------------------

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
        let transition_class = TransitionClass::from_bits_truncate(p.transition_class);

        let verifier = &*verifier;
        let mut seen_transitions = match verifier.seen_transitions.lock() {
            Ok(buffer) => buffer,
            Err(_) => return LaurnResult::Panic,
        };

        let ctx = VerificationContext {
            transition: &transition,
            raw_payload,
            signature: &signature,
            expected_input_state: expected_input_state,
            generated_output_state: generated_output_state,
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

        let result = verifier.inner.verify(&ctx);
        drop(ctx);

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

// ----------------------------------------------------------------------------
// Client Utilities (for Unreal Client to sign payloads)
// ----------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn laurn_diagnostic_sign_transition(
    raw_payload: *const u8,
    raw_payload_len: usize,
    out_signature: *mut [u8; 64],
) -> LaurnResult {
    catch_unwind_ffi(|| {
        if raw_payload.is_null() || out_signature.is_null() {
            return LaurnResult::NullPointer;
        }

        let payload = slice::from_raw_parts(raw_payload, raw_payload_len);

        // In a real scenario, the client loads its private key from disk/keystore.
        // Use deterministic diagnostic key for integration validation.
        let secret_bytes = [0x42; 32];
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);

        use ed25519_dalek::Signer;
        let sig = signing_key.sign(payload);

        ptr::copy_nonoverlapping(sig.to_bytes().as_ptr(), (*out_signature).as_mut_ptr(), 64);

        LaurnResult::Success
    })
}

// ----------------------------------------------------------------------------
// Replay Engine
// ----------------------------------------------------------------------------

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
            Ok(mut buf) => {
                buf.shrink_to_fit();
                *out_len = buf.len();
                let ptr = buf.as_mut_ptr();
                std::mem::forget(buf);
                *out_bytes = ptr;
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
                let mut buf = frame.raw_payload;
                buf.shrink_to_fit();
                *out_payload_len = buf.len();
                let ptr = buf.as_mut_ptr();
                std::mem::forget(buf);
                *out_payload = ptr;
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

                use replay::divergence::DivergenceReason::*;
                match report.reason {
                    ParentMismatch { expected, actual } => {
                        c_report.reason = LaurnDivergenceReason::ParentMismatch;
                        c_report.expected_commitment = expected.0;
                        c_report.actual_commitment = actual.0;
                    }
                    CommitmentMismatch { expected, actual } => {
                        c_report.reason = LaurnDivergenceReason::CommitmentMismatch;
                        c_report.expected_commitment = expected.0;
                        c_report.actual_commitment = actual.0;
                    }
                    EpochMismatch { expected, actual } => {
                        c_report.reason = LaurnDivergenceReason::EpochMismatch;
                        c_report.expected_epoch = expected.0;
                        c_report.actual_epoch = actual.0;
                    }
                    AuthorityMismatch { expected, actual } => {
                        c_report.reason = LaurnDivergenceReason::AuthorityMismatch;
                        c_report.expected_authority = expected.0;
                        c_report.actual_authority = actual.0;
                    }
                    PayloadMismatch => {
                        c_report.reason = LaurnDivergenceReason::PayloadMismatch;
                    }
                    LengthMismatch {
                        expected_frames,
                        actual_frames,
                    } => {
                        c_report.reason = LaurnDivergenceReason::LengthMismatch;
                        c_report.expected_frames = expected_frames;
                        c_report.actual_frames = actual_frames;
                    }
                    DecodeFailed => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_engine_lifecycle() {
        unsafe {
            let mut handle: *mut LaurnAuthorityEngineHandle = ptr::null_mut();

            assert_eq!(
                laurn_authority_engine_create(&mut handle as *mut _),
                LaurnResult::Success
            );
            assert!(!handle.is_null());

            assert_eq!(laurn_authority_engine_destroy(handle), LaurnResult::Success);
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

// ----------------------------------------------------------------------------
// Telemetry and Versioning
// ----------------------------------------------------------------------------

/// Returns the protocol version as an integer.
#[no_mangle]
pub extern "C" fn laurn_get_version() -> u32 {
    let version = version_crate::ProtocolVersion::current();
    ((version.major as u32) << 16) | ((version.minor as u32) << 8) | (version.patch as u32)
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
        let c_str = match std::ffi::CString::new(build_info) {
            Ok(s) => s,
            Err(_) => return LaurnResult::EncodeFailed,
        };
        let bytes = c_str.as_bytes_with_nul();

        if bytes.len() > buffer_len {
            return LaurnResult::BufferTooSmall;
        }

        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, bytes.len());
        }

        LaurnResult::Success
    })
}

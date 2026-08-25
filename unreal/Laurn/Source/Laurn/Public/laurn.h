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

#pragma once

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// ----------------------------------------------------------------------------
// Error Codes
// ----------------------------------------------------------------------------

typedef enum LaurnResult {
    LAURN_SUCCESS = 0,
    LAURN_NULL_POINTER = 1,
    LAURN_PANIC = 2,
    LAURN_OUT_OF_MEMORY = 3,
    LAURN_BUFFER_TOO_SMALL = 4,
    LAURN_DECODE_FAILED = 5,
    LAURN_VERIFICATION_FAILED = 6,
    LAURN_INVALID_CONFIG = 7,
    LAURN_ENCODE_FAILED = 8,
    LAURN_VERIFICATION_DUPLICATE = 9,
    LAURN_VERIFICATION_STATE_MISMATCH = 10,
    LAURN_END_OF_STREAM = 11,
    LAURN_DIVERGENCE_DETECTED = 12,
} LaurnResult;

// ----------------------------------------------------------------------------
// Opaque Handles
// ----------------------------------------------------------------------------

typedef struct LaurnAuthorityEngineHandle LaurnAuthorityEngineHandle;
typedef struct LaurnEpochEngineHandle LaurnEpochEngineHandle;
typedef struct LaurnPolicyEngineHandle LaurnPolicyEngineHandle;
typedef struct LaurnVerificationEngineHandle LaurnVerificationEngineHandle;
typedef struct LaurnTransitionHandle LaurnTransitionHandle;
typedef struct LaurnDeltaHandle LaurnDeltaHandle;
typedef struct LaurnMessageHandle LaurnMessageHandle;
typedef struct LaurnCanonicalStateHandle LaurnCanonicalStateHandle;
typedef struct LaurnPolicyHandle LaurnPolicyHandle;

// ----------------------------------------------------------------------------
// Authority Engine Operations
// ----------------------------------------------------------------------------

LaurnResult laurn_authority_engine_create(LaurnAuthorityEngineHandle** out_handle);
LaurnResult laurn_authority_engine_destroy(LaurnAuthorityEngineHandle* handle);
LaurnResult laurn_authority_engine_register_test_authority(LaurnAuthorityEngineHandle* handle);

// ----------------------------------------------------------------------------
// Epoch Engine Operations
// ----------------------------------------------------------------------------

LaurnResult laurn_epoch_engine_create(LaurnEpochEngineHandle** out_handle);
LaurnResult laurn_epoch_engine_destroy(LaurnEpochEngineHandle* handle);

// ----------------------------------------------------------------------------
// Policy Engine Operations
// ----------------------------------------------------------------------------

LaurnResult laurn_policy_engine_create(LaurnPolicyEngineHandle** out_handle);
LaurnResult laurn_policy_engine_destroy(LaurnPolicyEngineHandle* handle);

// ----------------------------------------------------------------------------
// Policy Operations
// ----------------------------------------------------------------------------

LaurnResult laurn_policy_create_default(LaurnPolicyHandle** out_policy);
LaurnResult laurn_policy_destroy(LaurnPolicyHandle* handle);

// ----------------------------------------------------------------------------
// Verification Engine Operations
// ----------------------------------------------------------------------------

LaurnResult laurn_verification_engine_create(LaurnVerificationEngineHandle** out_handle);
LaurnResult laurn_verification_engine_destroy(LaurnVerificationEngineHandle* handle);

// ----------------------------------------------------------------------------
// Protocol Decoding
// ----------------------------------------------------------------------------

LaurnResult laurn_protocol_decode_message(
    const uint8_t* buffer,
    size_t buffer_len,
    LaurnMessageHandle** out_message,
    size_t* out_bytes_consumed
);

LaurnResult laurn_protocol_encode_transition_message(
    uint32_t protocol_version,
    uint32_t transition_class,
    uint64_t transition_id,
    const uint8_t* raw_payload,
    size_t raw_payload_len,
    const uint8_t (*signature)[64],
    uint8_t** out_buffer,
    size_t* out_buffer_len
);

LaurnResult laurn_protocol_free_bytes(
    uint8_t* buffer,
    size_t buffer_len
);

LaurnResult laurn_message_destroy(LaurnMessageHandle* handle);

LaurnResult laurn_message_get_transition(
    const LaurnMessageHandle* message,
    LaurnTransitionHandle** out_transition
);

LaurnResult laurn_transition_destroy(LaurnTransitionHandle* handle);

LaurnResult laurn_message_get_signature(
    const LaurnMessageHandle* message,
    uint8_t (*out_signature)[64]
);

LaurnResult laurn_message_get_raw_payload(
    const LaurnMessageHandle* message,
    const uint8_t** out_payload,
    size_t* out_payload_len
);

LaurnResult laurn_message_get_protocol_version(
    const LaurnMessageHandle* message,
    uint32_t* out_version
);

LaurnResult laurn_transition_get_class(
    const LaurnTransitionHandle* transition,
    uint32_t* out_class
);

// ----------------------------------------------------------------------------
// State Commitment
// ----------------------------------------------------------------------------

LaurnResult laurn_state_commitment_compute(
    const uint8_t* buffer,
    size_t buffer_len,
    uint8_t (*out_hash)[32]
);

void laurn_free_bytes(uint8_t* ptr, size_t len);

// ----------------------------------------------------------------------------
// Verification
// ----------------------------------------------------------------------------

typedef struct LaurnVerificationParams {
    const LaurnTransitionHandle* transition;
    const uint8_t* raw_payload;
    size_t raw_payload_len;
    const uint8_t (*signature)[64];
    const uint8_t (*expected_input_state)[32];
    const uint8_t (*generated_output_state)[32];
    const LaurnAuthorityEngineHandle* authority_engine;
    const LaurnEpochEngineHandle* epoch_engine;
    const LaurnPolicyEngineHandle* policy_engine;
    const LaurnPolicyHandle* policy;
    uint64_t parent_state_timestamp_ms;
    bool has_evidence;
    uint32_t transition_protocol_version;
    uint32_t transition_class;
} LaurnVerificationParams;

LaurnResult laurn_verify_transition(
    const LaurnVerificationEngineHandle* verifier,
    const LaurnVerificationParams* params
);

// ----------------------------------------------------------------------------
// Client Utilities (for Unreal Client to sign payloads)
// ----------------------------------------------------------------------------

LaurnResult laurn_test_sign_transition(
    const uint8_t* raw_payload,
    size_t raw_payload_len,
    uint8_t (*out_signature)[64]
);
// ----------------------------------------------------------------------------
// Replay Engine
// ----------------------------------------------------------------------------

typedef struct LaurnReplayRecorderHandle LaurnReplayRecorderHandle;
typedef struct LaurnReplayReaderHandle LaurnReplayReaderHandle;

LaurnResult laurn_replay_recorder_create(
    const uint8_t (*initial_state)[32],
    LaurnReplayRecorderHandle** out_handle
);

LaurnResult laurn_replay_recorder_add_frame(
    LaurnReplayRecorderHandle* recorder,
    const uint8_t* raw_payload,
    size_t raw_payload_len,
    const uint8_t (*expected_output_state)[32]
);

LaurnResult laurn_replay_recorder_serialize(
    const LaurnReplayRecorderHandle* recorder,
    uint8_t** out_bytes,
    size_t* out_len
);

LaurnResult laurn_replay_recorder_destroy(
    LaurnReplayRecorderHandle* handle
);

LaurnResult laurn_replay_reader_create(
    const uint8_t* buffer,
    size_t buffer_len,
    LaurnReplayReaderHandle** out_handle
);

LaurnResult laurn_replay_reader_next_frame(
    LaurnReplayReaderHandle* reader,
    uint8_t** out_payload,
    size_t* out_payload_len,
    uint8_t (*out_expected_output_state)[32]
);

LaurnResult laurn_replay_reader_destroy(
    LaurnReplayReaderHandle* handle
);

typedef enum LaurnDivergenceReason {
    LAURN_DIVERGENCE_PARENT_MISMATCH = 0,
    LAURN_DIVERGENCE_COMMITMENT_MISMATCH = 1,
    LAURN_DIVERGENCE_EPOCH_MISMATCH = 2,
    LAURN_DIVERGENCE_AUTHORITY_MISMATCH = 3,
    LAURN_DIVERGENCE_PAYLOAD_MISMATCH = 4,
    LAURN_DIVERGENCE_LENGTH_MISMATCH = 5,
    LAURN_DIVERGENCE_DECODE_FAILED = 6,
} LaurnDivergenceReason;

typedef struct LaurnDivergenceReport {
    uint32_t frame_index;
    LaurnDivergenceReason reason;
    uint8_t expected_commitment[32];
    uint8_t actual_commitment[32];
    uint8_t expected_epoch[32];
    uint8_t actual_epoch[32];
    uint8_t expected_authority[32];
    uint8_t actual_authority[32];
    uint32_t expected_frames;
    uint32_t actual_frames;
} LaurnDivergenceReport;

LaurnResult laurn_replay_analyze_divergence(
    LaurnReplayReaderHandle* auth_reader,
    LaurnReplayReaderHandle* test_reader,
    LaurnDivergenceReport* out_report
);

#ifdef __cplusplus
}
#endif

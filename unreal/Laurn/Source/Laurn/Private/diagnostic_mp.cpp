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

#include <iostream>
#include <vector>
#include <cstring>
#include "laurn.h"

// Multiplayer integration diagnostic to verify we can link and call the verification logic.
// We simulate the flow of ALaurnPlayerController::ServerSubmitTransition.

int main()
{
    std::cout << "Starting Laurn Multiplayer Integration Diagnostic...\n";

    LaurnAuthorityEngineHandle* authority_engine = nullptr;
    if (laurn_authority_engine_create(&authority_engine) != LAURN_SUCCESS) {
        std::cerr << "Failed to create authority engine.\n";
        return 1;
    }
    
    // Register the diagnostic authority
    if (laurn_authority_engine_register_diagnostic_authority(authority_engine) != LAURN_SUCCESS) {
        std::cerr << "Failed to register diagnostic authority.\n";
        return 1;
    }

    LaurnEpochEngineHandle* epoch_engine = nullptr;
    if (laurn_epoch_engine_create(&epoch_engine) != LAURN_SUCCESS) {
        std::cerr << "Failed to create epoch engine.\n";
        return 1;
    }

    LaurnPolicyEngineHandle* policy_engine = nullptr;
    if (laurn_policy_engine_create(&policy_engine) != LAURN_SUCCESS) {
        std::cerr << "Failed to create policy engine.\n";
        return 1;
    }

    LaurnVerificationEngineHandle* verification_engine = nullptr;
    if (laurn_verification_engine_create(&verification_engine) != LAURN_SUCCESS) {
        std::cerr << "Failed to create verification engine.\n";
        return 1;
    }

    // Prepare an encoded diagnostic transition.
    // In a real flow, the Unreal Client calls laurn_protocol_encode_message.
    // For this diagnostic, we construct a real Transition Message.
    
    std::vector<uint8_t> raw_payload(32, 0x01);

    uint64_t transition_id = 1;
    uint32_t transition_class = 1;
    uint8_t authority_id[32] = {0};
    uint8_t epoch_id[32] = {0x01};
    uint64_t timestamp_ms = 1000;
    uint8_t input_state[32] = {0};
    uint8_t output_state[32] = {0};

    if (laurn_epoch_engine_register(
            epoch_engine,
            &epoch_id,
            1,
            0,
            2000,
            &input_state) != LAURN_SUCCESS) {
        std::cerr << "Failed to register diagnostic epoch.\n";
        return 1;
    }

    if (laurn_epoch_engine_activate(epoch_engine, &epoch_id) != LAURN_SUCCESS) {
        std::cerr << "Failed to activate diagnostic epoch.\n";
        return 1;
    }

    uint8_t signature[64] = {0};
    if (laurn_diagnostic_sign_transition(
            transition_id,
            transition_class,
            &epoch_id,
            timestamp_ms,
            &input_state,
            &output_state,
            raw_payload.data(),
            raw_payload.size(),
            &authority_id,
            &signature) != LAURN_SUCCESS) {
        std::cerr << "Failed to sign transition.\n";
        return 1;
    }

    uint8_t* encoded_bytes = nullptr;
    size_t encoded_size = 0;

    if (laurn_protocol_encode_transition_message(
            1, // protocol version
            transition_class,
            transition_id,
            &authority_id,
            &epoch_id,
            timestamp_ms,
            &input_state,
            &output_state,
            raw_payload.data(),
            raw_payload.size(),
            &signature,
            &encoded_bytes,
            &encoded_size) != LAURN_SUCCESS) {
        std::cerr << "Failed to encode transition message.\n";
        return 1;
    }
    
    std::cout << "Successfully encoded LaurnMessage of size " << encoded_size << " bytes.\n";
    
    // Now simulate the Server receiving it and verifying it:
    // (This mimics ULaurnSubsystem::VerifyIncomingTransition)
    
    LaurnMessageHandle* message_handle = nullptr;
    size_t bytes_consumed = 0;
    if (laurn_protocol_decode_message(encoded_bytes, encoded_size, &message_handle, &bytes_consumed) != LAURN_SUCCESS) {
        std::cerr << "Failed to decode message.\n";
        return 1;
    }
    
    LaurnTransitionHandle* transition_handle = nullptr;
    if (laurn_message_get_transition(message_handle, &transition_handle) != LAURN_SUCCESS) {
        std::cerr << "Failed to get transition from message.\n";
        return 1;
    }
    
    uint8_t rx_signature[64];
    laurn_message_get_signature(message_handle, &rx_signature);
    
    const uint8_t* rx_raw_payload = nullptr;
    size_t rx_raw_payload_len = 0;
    laurn_message_get_raw_payload(message_handle, &rx_raw_payload, &rx_raw_payload_len);
    
    uint32_t rx_protocol_version = 0;
    laurn_message_get_protocol_version(message_handle, &rx_protocol_version);
    
    uint32_t rx_transition_class = 0;
    laurn_transition_get_class(transition_handle, &rx_transition_class);
    
    // Compute a diagnostic output state
    uint8_t test_output_state[32] = {0};
    // (In reality this comes from laurn_state_commitment_compute)
    
    LaurnPolicyHandle* policy_handle = nullptr;
    laurn_policy_create_default(&policy_handle);
    
    LaurnVerificationParams params;
    std::memset(&params, 0, sizeof(params));
    params.transition = transition_handle;
    params.raw_payload = rx_raw_payload;
    params.raw_payload_len = rx_raw_payload_len;
    params.signature = &rx_signature;
    params.expected_input_state = &test_output_state;
    params.generated_output_state = &test_output_state;
    params.authority_engine = authority_engine;
    params.epoch_engine = epoch_engine;
    params.policy_engine = policy_engine;
    params.policy = policy_handle;
    params.parent_state_timestamp_ms = 0;
    params.has_evidence = false;
    params.transition_protocol_version = rx_protocol_version;
    params.transition_class = rx_transition_class;
    
    LaurnResult verify_res = laurn_verify_transition(verification_engine, &params);
    if (verify_res == LAURN_SUCCESS) {
        std::cout << "SUCCESS: Transition verified successfully!\n";
    } else {
        std::cout << "ERROR: Transition verification returned code " << verify_res << ".\n";
    }
    
    // Cleanup
    laurn_policy_destroy(policy_handle);
    laurn_transition_destroy(transition_handle);
    laurn_message_destroy(message_handle);
    laurn_protocol_free_bytes(encoded_bytes, encoded_size);
    laurn_verification_engine_destroy(verification_engine);
    laurn_policy_engine_destroy(policy_engine);
    laurn_epoch_engine_destroy(epoch_engine);
    laurn_authority_engine_destroy(authority_engine);

    std::cout << "Laurn Multiplayer Integration Diagnostic completed.\n";
    return 0;
}

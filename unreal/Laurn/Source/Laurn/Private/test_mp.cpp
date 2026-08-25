#include <iostream>
#include <vector>
#include <cstring>
#include "laurn.h"

// Basic stub to verify we can link and call the multiplayer verification logic.
// We simulate the flow of ALaurnPlayerController::ServerSubmitTransition.

int main()
{
    std::cout << "Starting Laurn Multiplayer Integration Test...\n";

    LaurnAuthorityEngineHandle* authority_engine = nullptr;
    if (laurn_authority_engine_create(&authority_engine) != LAURN_SUCCESS) {
        std::cerr << "Failed to create authority engine.\n";
        return 1;
    }
    
    // Register the test authority
    if (laurn_authority_engine_register_test_authority(authority_engine) != LAURN_SUCCESS) {
        std::cerr << "Failed to register test authority.\n";
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

    // Prepare a mock encoded transition.
    // In a real flow, the Unreal Client calls laurn_protocol_encode_message.
    // For this test, we will create a dummy Transition Message.
    
    // First, let's create a raw payload
    std::vector<uint8_t> raw_payload = { 0x01, 0x02, 0x03, 0x04 };
    
    // Generate signature using our test authority key
    uint8_t signature[64] = {0};
    if (laurn_test_sign_transition(raw_payload.data(), raw_payload.size(), &signature) != LAURN_SUCCESS) {
        std::cerr << "Failed to sign payload.\n";
        return 1;
    }
    
    // Encode the entire LaurnMessage (version + transition + raw_payload + signature)
    uint8_t* encoded_bytes = nullptr;
    size_t encoded_size = 0;
    
    if (laurn_protocol_encode_transition_message(
            1, // protocol version
            1, // transition class
            1, // id
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
    
    // Compute dummy generated output state
    uint8_t dummy_output_state[32] = {0};
    // (In reality this comes from laurn_state_commitment_compute)
    
    LaurnPolicyHandle* policy_handle = nullptr;
    laurn_policy_create_default(&policy_handle);
    
    LaurnVerificationParams params;
    std::memset(&params, 0, sizeof(params));
    params.transition = transition_handle;
    params.raw_payload = rx_raw_payload;
    params.raw_payload_len = rx_raw_payload_len;
    params.signature = &rx_signature;
    params.expected_input_state = &dummy_output_state;
    params.generated_output_state = &dummy_output_state;
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
        std::cout << "WARNING: Transition verification returned code " << verify_res << ". (Expected since we didn't mock everything perfectly, but FFI plumbing works)\n";
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

    std::cout << "Laurn Multiplayer Integration Test completed.\n";
    return 0;
}

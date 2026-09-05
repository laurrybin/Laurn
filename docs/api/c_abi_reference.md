# C ABI Reference

Status: Pre-alpha.

The LAURN Rust core exposes a handle-based C ABI through `unreal/Laurn/Source/Laurn/Public/laurn.h`. Consumers create and destroy the engines they use; initialization is not global.

## Engine lifecycle

The authority, epoch, policy, and verification engines use explicit create and destroy functions. Every successfully created opaque handle is caller-owned until its matching destroy function is called.

Replay protection belongs to the verification-engine handle, so duplicate detection persists across verification calls made with the same verifier.

## Authority and diagnostics

`LaurnAuthorityEngineHandle` stores the authorities used for signature and capability checks.

`laurn_authority_engine_register_diagnostic_authority` registers the deterministic diagnostic authority exposed by the current ABI. It exists for diagnostics and integration testing, not production authority provisioning.

`laurn_diagnostic_sign_transition` signs the canonical transition representation with the matching deterministic diagnostic key. It is not a production signing or key-management API.

## Epoch lifecycle

The epoch engine exposes:

- `laurn_epoch_engine_register`
- `laurn_epoch_engine_activate`
- `laurn_epoch_engine_close`

Registration supplies the epoch identifier, sequence, time window, and initial state commitment. Verification checks the transition epoch against the configured epoch engine and active time window.

## Policy

`laurn_policy_engine_create` creates the policy engine. `laurn_policy_create_default` returns the currently exposed default policy object; release it with `laurn_policy_destroy`.

## Protocol and transitions

`laurn_protocol_decode_message` decodes a LAURN message. `laurn_protocol_encode_transition_message` encodes a transition message from explicit protocol, transition, authority, epoch, commitment, payload, and signature fields.

Decoded messages expose the transition, signature, raw payload, and protocol version. Decoded transitions expose transition class and timestamp.

Transition class is part of the signed representation. Unknown class bits are rejected during verification.

Destroy message and transition handles with their matching destroy functions.

The raw payload pointer returned by `laurn_message_get_raw_payload` is borrowed from the message and must not outlive that message handle.

Buffers returned by `laurn_protocol_encode_transition_message` must be released with `laurn_protocol_free_bytes`.

## State commitments

`laurn_state_commitment_compute` computes a 32-byte domain-separated BLAKE3 state commitment from caller-supplied bytes.

The function does not canonicalize arbitrary application objects. The caller is responsible for producing canonical bytes before commitment.

## Verification

Hosts populate `LaurnVerificationParams` and call `laurn_verify_transition`.

The verification context supplies the decoded transition, raw payload, signature, expected input-state commitment, host-generated output-state commitment, authority and epoch engines, policy state, parent timestamp, evidence flag, protocol version, and transition class.

Verification coordinates authority authentication, epoch validation, policy checks, parent-state continuity, output-state commitment comparison, protocol-version checks, transition-class checks, and replay protection.

LAURN does not execute application-specific gameplay or simulation logic to produce `generated_output_state`; the host supplies that commitment.

After `LAURN_SUCCESS`, the transition identifier is inserted into replay protection for that verification-engine handle. Re-submitting the same identifier to the same verifier returns `LAURN_VERIFICATION_DUPLICATE`.

## Replay

The replay ABI exposes recorder, reader, and divergence-analysis operations.

A recorder stores an initial state commitment and frames containing raw payload bytes plus expected output-state commitments. Serialized replay buffers returned by the ABI are caller-owned and must be released with `laurn_free_bytes`.

`laurn_replay_reader_create` reads from a caller-supplied replay buffer. That buffer must remain valid for the lifetime of the reader.

Payload buffers returned by `laurn_replay_reader_next_frame` are caller-owned and must be released with `laurn_free_bytes`.

`laurn_replay_analyze_divergence` reports the first detected frame-count, payload, output-state commitment, or decode difference. When the differing frame payloads decode as serialized `LaurnMessage` values, the analyzer can additionally classify parent-state, epoch, and authority differences.

Semantic classification therefore depends on what the caller records as the replay frame payload. A host that records only the inner application payload does not provide the protocol metadata needed for those classifications.

## ABI status

The C ABI is pre-alpha. Current adapters and tests use this interface, but binary and source compatibility are not yet guaranteed across releases.

Target integration work includes production authority provisioning, release packaging, and additional host adapters while preserving the explicit C-compatible ownership boundary.

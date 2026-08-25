# LAURN API Reference

## Unreal Engine C++ API

### `ULaurnSubsystem`
The primary engine subsystem for managing the LAURN runtime.

- **`void InitializeRuntime()`**
  Initializes the Rust FFI bindings and allocates the replay buffer. Must be called before any other operations.
  
- **`void AdvanceEpoch()`**
  Advances the deterministic timeline. Hashes all registered components and commits a state delta.

- **`void RecordTransition(const FLaurnTransition& Transition)`**
  Records a state mutation into the replay buffer.

- **`FString GetCurrentCommitmentHash() const`**
  Returns the BLAKE3 hash of the current canonical state.

### `ULaurnStateComponent`
An Actor Component attached to entities whose state must be verified.

- **`void RegisterState(const FString& Key, const TArray<uint8>& Data)`**
  Registers an opaque byte array against a specific key in the LAURN state machine.

## Rust FFI API (`bindings/c`)

For developers building non-Unreal clients (e.g., dedicated Linux servers or C# Unity clients), the following C ABI functions are exported:

- **`int32_t laurn_runtime_create(LaurnRuntime** out_runtime)`**
  Allocates a new verification runtime. Returns 0 on success.

- **`int32_t laurn_epoch_advance(LaurnRuntime* runtime)`**
  Advances the epoch and returns the resulting state hash internally.

- **`int32_t laurn_record_transition(LaurnRuntime* runtime, const uint8_t* data, size_t len)`**
  Records an arbitrary byte payload as a transition in the current epoch.

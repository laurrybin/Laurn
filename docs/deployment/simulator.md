# LAURN Simulator and Inspector

The `tools/` workspace contains command-line utilities for replay, evidence, and inspection paths without requiring Unreal Engine.

These are development and diagnostic utilities, not a production deployment environment.

## `laurn-simulator`

The simulator runs a fixed synthetic replay-divergence scenario.

The current scenario:

- creates three deterministic server-node fixtures from fixed seeds
- runs up to 100 synthetic epochs
- constructs signed LAURN transition messages
- records equivalent replay frames for the reference nodes
- injects an output-state commitment mismatch into server 2 at epoch 75
- creates and signs a server execution-evidence record
- verifies the evidence issuer signature
- compares the reference and modified replay streams
- checks that divergence analysis reports the injected commitment mismatch

The simulator exits after the injected divergence is detected. It does not currently accept seed or epoch-count command-line options.

### Usage

```bash
cargo run --bin laurn-simulator
```

### Scope

The simulator exercises transition serialization and signing, replay recording, evidence-signature verification, and divergence analysis.

It does not run a complete host-configured `VerificationEngine` acceptance path, does not verify SGX or AWS Nitro hardware attestation, and is not a performance benchmark.

## `laurn-inspector`

The inspector reads serialized LAURN replay files and provides three subcommands.

### Dump a replay

```bash
cargo run --bin laurn-inspector -- dump session_001.laurn
```

`dump` prints the replay header and decodes transition frames where possible, including transition identifier, epoch, authority, input state, timestamp, and expected output-state commitment.

### Run verifier diagnostics

```bash
cargo run --bin laurn-inspector -- verify session_001.laurn
```

`verify` decodes each transition and runs the verification engine against the inspector current in-process configuration.

The current command creates empty authority and epoch engines and does not load authority registration or active epoch configuration from the replay file. A replay produced by a configured host can therefore report verification failures because the inspector lacks that external context.

This command is useful for verifier diagnostics but is not equivalent to re-validating a host session with its original authority, epoch, policy, and state configuration.

### Compare two replays

```bash
cargo run --bin laurn-inspector -- diverge reference.laurn test.laurn
```

`diverge` compares two replay streams and reports the first detected difference. Frame-count, payload, output-state commitment, and decode differences are available directly. When differing frame payloads decode as serialized `LaurnMessage` values, the analyzer can additionally distinguish epoch, authority, and parent-state differences.

## Target tooling

Planned tooling includes configurable authority, epoch, and policy context for inspector verification, richer replay metadata, and domain-specific reconstruction support.

Those items are not part of the current command-line behavior.

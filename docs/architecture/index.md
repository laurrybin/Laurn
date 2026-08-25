# Architecture Overview

LAURN is a deterministic state-transition runtime. It is fundamentally designed around the idea that game/simulation state should be verifiable, reproducible, and mathematically provable without relying on non-deterministic host environments.

## Core Concepts

### Authority
In LAURN, every state transition requires authorization. The `core/authority` module defines cryptographic identities (typically ED25519 or secp256k1 keypairs) that sign incoming transitions. The engine verifies these signatures against access control policies before evaluating logic.

### Epochs
Time in LAURN progresses in discrete chunks known as **Epochs** (`core/epoch`). An epoch represents a single, monolithic advancement of the simulation frame. State mutations are grouped by epoch, ensuring that order-of-operations remains strict across all executing nodes.

### Canonical State and Deltas
The entire simulation state is canonicalized into a deterministic memory layout (`core/state`). When a transition occurs, it generates a **Delta** (`core/delta`), representing the exact semantic change to the state. 

### Commitments
At the end of every epoch, LAURN aggregates the state and produces a **Cryptographic Commitment** (`core/commitment`). This is a deterministic hash (e.g., BLAKE3) of the canonical state. If two LAURN instances process the exact same transitions, their State Commitments are guaranteed to match bit-for-bit.

### Evidence and Replay
LAURN generates **Execution Evidence** for every state modification (`core/evidence`). This evidence, alongside the state deltas, is logged into a Replay Buffer (`core/replay`). This allows external verifiers to reconstruct the entire history of a simulation and validate that the final State Commitment was reached legitimately.

# ADR 0001: Use Rust for Core Logic

**Date:** 2026-08-25  
**Status:** Accepted

## Context

LAURN needs an engine-agnostic core for cryptographic verification, deterministic serialization, replay handling, policy evaluation, and state commitments.

The core also crosses an FFI boundary into host engines such as Unreal Engine, so implementation choices must support explicit ownership and a stable C-compatible interface.

## Decision

Implement the engine-agnostic LAURN core in Rust and expose host-facing functionality through the C ABI.

## Consequences

- **Positive:** Safe Rust provides strong memory-safety and data-race protections for code that remains inside its guarantees.
- **Positive:** Rust provides mature libraries for Ed25519 signatures, BLAKE3 hashing, Borsh serialization, fixed-point arithmetic, testing, and fuzzing.
- **Positive:** The core can remain independent of Unreal Engine headers and UnrealBuildTool.
- **Positive:** Rust types can enforce explicit representations before data crosses the FFI boundary.
- **Negative:** `unsafe` code and C ABI boundaries still require manual review; Rust does not make foreign callers or pointer use automatically safe.
- **Negative:** The project must maintain C-compatible ownership, allocation, and lifetime rules.
- **Negative:** Engine developers working primarily in C++ must understand the Rust/C ABI boundary when debugging integration issues.

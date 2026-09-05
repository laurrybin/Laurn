# ADR 0002: Unreal Engine C ABI Integration

**Date:** 2026-08-25  
**Status:** Accepted

## Context

Unreal Engine is a primary LAURN host target and operates natively in C++.

The Rust core should remain independent of Unreal headers and build tooling while still exposing verification, commitment, replay, and lifecycle operations to Unreal.

## Decision

Expose a flat C ABI from `bindings/c` and consume that ABI from the Unreal plugin under `unreal/Laurn`.

Opaque handles represent Rust-owned engine state. C-compatible parameters and explicit create, destroy, and free functions define the ownership boundary.

## Consequences

- **Positive:** The Rust core remains decoupled from Unreal Engine headers and UnrealBuildTool.
- **Positive:** The same ABI design can support other native host integrations without requiring those hosts to depend on Rust types directly.
- **Positive:** Ownership and lifecycle operations are explicit at the language boundary.
- **Negative:** FFI calls require manual pointer, lifetime, allocation, and error-handling discipline.
- **Negative:** C-compatible representations add serialization and adapter boilerplate.
- **Negative:** ABI compatibility must be managed deliberately as the pre-alpha interface evolves.
- **Negative:** Host-specific lifecycle concerns, such as Unreal epoch orchestration and packaging, still require adapter code above the C ABI.

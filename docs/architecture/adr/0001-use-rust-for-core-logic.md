# ADR 0001: Use Rust for Core Logic

**Date:** 2026-08-25  
**Status:** Accepted  

## Context
LAURN requires an engine-agnostic core capable of strict determinism, high performance, and safe memory management. While C++ is the industry standard for real-time engines (like Unreal Engine), writing highly deterministic and mathematically rigorous cross-platform code in C++ is notoriously difficult due to undefined behavior and compiler-specific optimizations.

## Decision
The core logic of LAURN will be implemented entirely in **Rust**.

## Consequences
- **Positive:** Rust's strict compiler guarantees prevent data races and memory corruption.
- **Positive:** Rust provides excellent tooling for deterministic compilation (e.g., `no_std` environments).
- **Positive:** Rust allows for easy integration of high-quality cryptographic libraries.
- **Negative:** Requires creating and maintaining a robust C ABI (`bindings/c`) to communicate with host engines like Unreal.
- **Negative:** Increases the learning curve for engine developers solely familiar with C++.

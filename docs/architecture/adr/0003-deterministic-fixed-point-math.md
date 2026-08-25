# ADR 0003: Deterministic Fixed-Point Math

**Date:** 2026-08-25  
**Status:** Accepted  

## Context
Floating-point math (`f32`, `f64`) behaves differently across CPU architectures (x86 vs ARM), compiler optimization levels (e.g., `-ffast-math`), and operating systems. For LAURN's state commitments to match identically across all verifying nodes, the simulation state cannot drift by even a single bit.

## Decision
Floating-point math is strictly forbidden within the LAURN core state and transition evaluation logic. We will implement and use a custom fixed-point mathematics engine (`core/math`).

## Consequences
- **Positive:** Guarantees bit-for-bit determinism across all platforms (Windows, Linux, Consoles, Mobile).
- **Negative:** Fixed-point math incurs a slight performance overhead compared to hardware-accelerated floating-point operations.
- **Negative:** Game developers integrating LAURN must convert their engine's native floating-point representations (e.g., Unreal's `FVector`) to LAURN's fixed-point structures at the FFI boundary, which requires explicit quantization logic.

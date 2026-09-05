# ADR 0003: Deterministic Numeric Representations

**Date:** 2026-08-25  
**Status:** Accepted

## Context

State commitments depend on stable byte representations. Native floating-point values can introduce problematic representations such as NaN, infinity, and negative zero, while host simulations may also differ in how floating-point operations are evaluated.

LAURN therefore needs explicit numeric representations at the verified-state boundary.

## Decision

Use deterministic numeric representations in `core/math`:

- `CanonicalF32` and `CanonicalF64` reject non-finite values and normalize negative zero before serialization.
- fixed-point vector, quaternion, and transform types use `I48F16`.
- host integrations quantize or canonicalize engine-native values before including them in canonical state.

The choice of representation is explicit rather than treating unrestricted host floating-point state as canonical by default.

## Consequences

- **Positive:** Canonical wrappers remove NaN, infinity, and negative-zero ambiguity from serialized floating-point values.
- **Positive:** Fixed-point vector and transform types provide a stable integer-backed representation after quantization.
- **Positive:** The verified-state-boundary makes numeric conversion rules visible to host integrations.
- **Negative:** Quantization introduces finite precision and requires applications to choose appropriate scales and ranges.
- **Negative:** Host integrations must convert engine-native numeric types before commitment.
- **Negative:** These primitives do not by themselves prove deterministic behavior of an entire simulation across every CPU, compiler, operating system, or engine configuration.

# Troubleshooting LAURN

## Verification Failures

**Symptom**: `laurn_verify_epoch()` returns `-4` (Verification Failure) or Unreal logs show `LAURN ERROR: State Hash Mismatch`.

**Cause**: 
The state recomputed by the LAURN engine during transition replay does not mathematically match the State Commitment claimed by the authoritative server.

**Solutions**:
1. **Floating Point Non-Determinism**: LAURN heavily advises against relying on floating point mathematics (like `FVector` or `FRotator` directly computed via physics) for authoritative state. Switch to the provided `laurn::fixed` fixed-point libraries.
2. **Missing State Transitions**: Ensure that every single action that modifies a registered `ULaurnStateComponent` is explicitly wrapped in a `LaurnSubsystem->RecordTransition()` call.
3. **Random Number Generators (RNG)**: Ensure all gameplay RNG uses a deterministic, seeded generator tied to the `EpochId`, not `FMath::Rand()`.

## Platform Integrity Errors

**Symptom**: Execution Evidence generation fails with `UnsupportedPlatform`.

**Cause**: 
LAURN refuses to fake hardware attestations. You are running the verification engine on a standard developer machine (Windows/Mac) instead of an AWS Nitro Enclave or Intel SGX node.

**Solutions**:
- During local development, rely on standard Ed25519 signatures from your server authority.
- Only attempt hardware attestation when deployed to a physical TEE.

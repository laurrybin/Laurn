# LAURN Protocol Specification

The LAURN engine relies on binary determinism across all platforms. Data sent over the wire must be serialized using the [Borsh (Binary Object Representation Serializer for Hashing)](https://borsh.io/) specification.

## Core Primitives

- `StateCommitment`: `[u8; 32]` (BLAKE3 Hash of the domain-separated canonical state).
- `EpochId`: `[u8; 32]` (Unique identifier for a time slice).
- `AuthorityId`: `[u8; 32]` (Ed25519 Public Key).

## Network Messages

All messages on the wire are prepended with a 1-byte message type discriminator.

### 1. State Delta (Type 0x01)
Sent by authorities to declare state changes.
- `EpochId` (32 bytes)
- `StateCommitment` (32 bytes)
- `u32` length of operations
- Array of `DeltaOp`

### 2. Execution Evidence (Type 0x02)
Sent by Trusted Execution Environments to attest a state transition.
- `EvidenceId` (32 bytes)
- `EvidenceType` (1 byte enum: 0=Server, 1=SGX, 2=Nitro)
- `AuthorityId` (32 bytes)
- `EpochId` (32 bytes)
- `u64` timestamp_ms (Little-Endian)
- `TransitionCommitment` (32 bytes)
- `u32` length of raw_attestation
- Array of `u8` (raw attestation payload)
- `Signature` (64 bytes Ed25519)

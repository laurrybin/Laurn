# Security Policy

## Supported Versions

Currently, LAURN is in early development. When a stable version is released, security patches will be applied to the latest minor versions of the `1.x` release line.

## Threat Model

LAURN explicitly models trust boundaries. We provide cryptographically verifiable state-transition evidence within a defined trust boundary. 

We do NOT claim:
- Universal "unhackable" anti-cheat.
- Automatic determinism of arbitrary third-party code.

We DO claim:
- If a state is declared deterministic, and transitions are correctly implemented, LAURN will definitively flag unauthorized or computationally invalid transitions.

## Reporting a Vulnerability

If you discover a security vulnerability within LAURN (e.g., a cryptographic flaw, verification bypass, epoch confusion, parser exploit, or memory corruption via FFI), please report it directly to the maintainers privately rather than opening a public issue.

(Note: Security reporting email address to be established).

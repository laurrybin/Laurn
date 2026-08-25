# Contributing to LAURN

## Global Development Doctrine

LAURN follows a strict "Production from First Implementation" development doctrine.

1. **No Placeholders**: We do not use mocks, stubs, fake implementations, or placeholders (like `TODO`, `FIXME`, `unimplemented!()`, `todo!()`) in production code. A feature is implemented correctly when first introduced. If it's too large, split it into vertically integrated, independent, production-valid slices.
2. **Vertical Integration**: Every feature must integrate into the actual architecture (protocol -> Rust core -> FFI -> Unreal adapter).
3. **No Architectural Debt**: Do not introduce temporary architecture to meet a milestone (e.g., no `DummyVerifier` or `PlaceholderTransport`).
4. **Claim Discipline**: Every claim (e.g., "cross-platform", "deterministic") must be backed by real tests and evidence.
5. **Definition of Done**: A feature is done when its implementation exists, is integrated, tests pass, failure paths exist, and no prohibited placeholders remain.

## Commits & PRs
- Ensure you run the placeholder audit script or rely on CI to ensure no forbidden keywords are in your PR.
- Run `cargo fmt` and `cargo clippy --workspace -- -D warnings` before submitting.
- Follow conventional commits.

Thank you for helping us build a robust, verifiable state-transition engine!

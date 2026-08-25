# Contributing to LAURN

We welcome contributions to LAURN. To maintain the integrity and determinism of the state engine, all contributions must adhere to strict engineering and architectural standards.

## Contribution Workflow (GitHub Flow)

1. **Fork and Clone**: Fork the repository and clone it locally.
2. **Branch Naming**: Use the format `<type>/<issue-number>-<brief-description>` (e.g., `feat/12-add-fixed-point-sqrt`).
3. **Commit Messages**: All commits must follow the [Conventional Commits](https://www.conventionalcommits.org/) standard.
4. **Pull Requests**: Open a Pull Request against the `main` branch. Ensure the CI pipeline passes.

## Architectural Guidelines

- **Determinism**: The core state engine (`/core/*`) must remain strictly deterministic. Never use floating-point types (`f32`, `f64`) or standard library components that introduce non-determinism (e.g., non-seeded RNGs or system time). Use the provided fixed-point math engine.
- **No Panics**: The core logic should return `Result` types. Usage of `unwrap()`, `expect()`, or `panic!()` is strictly forbidden in production code outside of test harnesses.
- **C ABI Stability**: Any changes to `/bindings/c/` must maintain backwards compatibility or be explicitly version-bumped following semantic versioning.

## Development Setup

```bash
# Install the required Rust toolchain
rustup toolchain install 1.75.0

# Install clippy and rustfmt
rustup component add clippy rustfmt

# Run local checks before pushing
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
```

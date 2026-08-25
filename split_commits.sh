#!/bin/bash
set -e

# Reset to remove the monolithic commit but keep files staged/unstaged
git reset HEAD~1

# 1. Initialize workspace and foundational config
git add Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml deny.toml .gitignore .github/ README.md LICENSE SECURITY.md CONTRIBUTING.md
git commit -m "chore(repo): initialize workspace and foundational config"

# 2. Version system
git add core/version/
git commit -m "feat(version): add protocol versioning system"

# 3. Math engine
git add core/math/
git commit -m "feat(math): implement fixed-point math engine"

# 4. Authority
git add core/authority/
git commit -m "feat(authority): implement cryptographic authority identities"

# 5. Epoch progression
git add core/epoch/
git commit -m "feat(epoch): implement deterministic time and epoch progression"

# 6. Commitment
git add core/commitment/
git commit -m "feat(commitment): implement cryptographic state commitments"

# 7. State representation
git add core/state/
git commit -m "feat(state): implement canonical state representation"

# 8. Transitions
git add core/transition/
git commit -m "feat(transition): implement deterministic state transitions"

# 9. Semantic state deltas
git add core/delta/
git commit -m "feat(delta): implement semantic state deltas"

# 10. Execution evidence
git add core/evidence/
git commit -m "feat(evidence): implement execution evidence and platform integrity"

# 11. Access control / policy
git add core/policy/
git commit -m "feat(policy): implement access control and rule evaluation"

# 12. Runtime layer
git add core/runtime/
git commit -m "feat(runtime): implement execution runtime layer"

# 13. Replay & divergence
git add core/replay/
git commit -m "feat(replay): implement replay recorder and divergence analysis"

# 14. Verification
git add core/verification/
git commit -m "feat(verification): implement evidence verification and replay buffer"

# 15. Protocol
git add protocol/
git commit -m "feat(protocol): implement network communication protocol and serialization"

# 16. FFI
git add bindings/c/
git commit -m "feat(ffi): implement C bindings and logging callbacks"

# 17. Simulator
git add tools/simulator/
git commit -m "feat(simulator): implement end-to-end system validation simulator"

# 18. Tools (Replay & Inspector)
git add tools/replay/ tools/inspector/
git commit -m "feat(tools): implement replay runner and state inspector"

# 19. Benchmarks
git add tools/benchmarks/
git commit -m "perf(benchmarks): add benchmark suite for core engine"

# 20. Fuzzing
git add fuzz/
git commit -m "test(fuzz): add fuzzing targets for delta and message components"

# 21. Unreal module
git add unreal/
git commit -m "feat(unreal): implement Unreal Engine integration module"

# 22. Unreal example
git add examples/unreal/
git commit -m "docs(unreal): provide Unreal Engine example project"

# 23. Architecture docs
git add docs/
git commit -m "docs(architecture): add comprehensive architecture and integration documentation"

# 24. C++ Tests
git add test_link.cpp test_quantization.cpp test_link test_quantization
git commit -m "test(cpp): add C++ test harnesses for quantization and linking"

echo "Done!"

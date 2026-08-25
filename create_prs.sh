#!/bin/bash
set -e

# Record the HEAD which is the top of the 24 commits
TOP_COMMIT=$(git rev-parse HEAD)
ROOT_COMMIT=$(git rev-parse HEAD~24)

echo "Top commit: $TOP_COMMIT"
echo "Root commit: $ROOT_COMMIT"

# Create branches at the respective group boundaries
git branch b8-docs $TOP_COMMIT
git branch b7-unreal HEAD~2
git branch b6-tools HEAD~4
git branch b5-ffi HEAD~8
git branch b4-verification HEAD~9
git branch b3-state HEAD~12
git branch b2-core HEAD~18
git branch b1-repo HEAD~23

# Reset main to the root commit
git reset --hard $ROOT_COMMIT

# Force push main to remote (clears the monolithic commit)
git push -f origin main

# Push all branches
git push -u origin b1-repo b2-core b3-state b4-verification b5-ffi b6-tools b7-unreal b8-docs

# Create stacked PRs
gh pr create --base main --head b1-repo --title "[Group 1] Repository Configuration" --body "Initializes the workspace and foundational configs."
gh pr create --base b1-repo --head b2-core --title "[Group 2] Core Primitives" --body "Implements math, authority, epoch, and commitments."
gh pr create --base b2-core --head b3-state --title "[Group 3] State & Execution Engine" --body "Implements state, transition, delta, evidence, policy, and runtime."
gh pr create --base b3-state --head b4-verification --title "[Group 4] Verification & Networking" --body "Implements replay, verification, and network protocol."
gh pr create --base b4-verification --head b5-ffi --title "[Group 5] Foreign Function Interface" --body "Implements C bindings and logging callbacks."
gh pr create --base b5-ffi --head b6-tools --title "[Group 6] Tooling" --body "Implements simulator, inspector, benchmarks, and fuzzing."
gh pr create --base b6-tools --head b7-unreal --title "[Group 7] Engine Integration & Examples" --body "Implements Unreal Engine module and examples."
gh pr create --base b7-unreal --head b8-docs --title "[Group 8] Documentation & Final Tests" --body "Adds architecture docs and C++ tests."

echo "All PRs created successfully!"

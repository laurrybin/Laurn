#!/bin/bash
set -e

echo "Fast-forwarding main to the top of our commit stack (b8-docs)..."
git checkout main
git reset --hard b8-docs

# Force push to overwrite the single merge commit created by the first PR
git push -f origin main

echo "Deleting remote branches..."
git push origin --delete b2-core b3-state b4-verification b5-ffi b6-tools b7-unreal b8-docs || true
git push origin --delete b1-repo || true

echo "Deleting local branches..."
git branch -D b1-repo b2-core b3-state b4-verification b5-ffi b6-tools b7-unreal b8-docs || true

echo "Done!"

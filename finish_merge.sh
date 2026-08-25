#!/bin/bash
set -e

echo "Fast-forwarding main to the top of our commit stack (b8-docs)..."
git checkout main
git reset --hard b8-docs
git push origin main

echo "Deleting remote branches..."
git push origin --delete b2-core b3-state b4-verification b5-ffi b6-tools b7-unreal b8-docs || true

echo "Deleting local branches..."
git branch -D b2-core b3-state b4-verification b5-ffi b6-tools b7-unreal b8-docs || true

echo "Done!"

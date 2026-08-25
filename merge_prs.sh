#!/bin/bash
set -e

# Checkout main and pull latest just in case
git checkout main
git pull origin main || true

# Merge PRs sequentially using gh
echo "Merging PRs..."
gh pr merge b1-repo --merge --delete-branch
gh pr merge b2-core --merge --delete-branch
gh pr merge b3-state --merge --delete-branch
gh pr merge b4-verification --merge --delete-branch
gh pr merge b5-ffi --merge --delete-branch
gh pr merge b6-tools --merge --delete-branch
gh pr merge b7-unreal --merge --delete-branch
gh pr merge b8-docs --merge --delete-branch

# Pull the final merged main branch
git pull origin main

echo "All PRs merged and branches deleted."

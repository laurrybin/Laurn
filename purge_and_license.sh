#!/bin/bash
set -e

# Stash our uncommitted new documentation files
git stash -u

# 1. Purge the old documentation commit from history
BAD_COMMIT=$(git log --format="%H" --grep="docs(architecture): add comprehensive architecture" -n 1)
if [ -n "$BAD_COMMIT" ]; then
    echo "Dropping old documentation commit: $BAD_COMMIT"
    GIT_SEQUENCE_EDITOR="sed -i -e 's/^pick \('${BAD_COMMIT:0:7}'\|${BAD_COMMIT}\)/drop \1/'" git rebase -i HEAD~5
else
    echo "Old documentation commit not found. Skipping rebase."
fi

# Restore our new documentation files
git stash pop || true

# 2. Prepend Apache 2.0 license to all source files
LICENSE_TEXT="// Copyright 2026 laurrybin and Laurn Contributors
//
// Licensed under the Apache License, Version 2.0 (the \"License\");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an \"AS IS\" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
"

# Create a temporary file with the license text
echo "$LICENSE_TEXT" > /tmp/license_header.txt

# Find all source files and prepend the license if they don't already have it
find . -type f \( -name "*.rs" -o -name "*.cpp" -o -name "*.h" -o -name "*.cs" \) \
    -not -path "*/target/*" \
    -not -path "*/Binaries/*" \
    -not -path "*/Intermediate/*" \
    -not -path "*/.git/*" | while read filepath; do
    if ! grep -q "Licensed under the Apache License" "$filepath"; then
        cat /tmp/license_header.txt "$filepath" > /tmp/temp_file
        mv /tmp/temp_file "$filepath"
    fi
done

# Commit the license headers
git add .
# Unstage the new documentation files so we can commit them atomically
git reset HEAD docs/ README.md CONTRIBUTING.md SECURITY.md CODE_OF_CONDUCT.md CHANGELOG.md .github/ || true
git commit -m "chore(license): add Apache 2.0 headers to all source files"

# 3. Atomic Documentation Commits
git add README.md CONTRIBUTING.md SECURITY.md CODE_OF_CONDUCT.md CHANGELOG.md
git commit -m "docs(root): add standard open-source governance files"

git add .github/
git commit -m "docs(templates): add GitHub issue and PR templates"

git add docs/architecture/index.md docs/architecture/adr/
git commit -m "docs(architecture): introduce ADRs and core architectural concepts"

git add docs/api/
git commit -m "docs(api): document FFI boundaries and Unreal integration"

git add docs/deployment/
git commit -m "docs(deployment): add CLI simulator documentation"

# 4. Force push
echo "Force pushing to main..."
git push -f origin main

echo "Done!"

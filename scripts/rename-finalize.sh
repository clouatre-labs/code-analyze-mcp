#!/usr/bin/env bash
# Post-merge: rename the GitHub repository from code-analyze-mcp to aptu-coder.
# Idempotent: skips rename if the repository is already named aptu-coder.
set -euo pipefail

if gh api repos/clouatre-labs/aptu-coder --silent 2>/dev/null; then
  echo "Repository already renamed to aptu-coder. Nothing to do."
  exit 0
fi

echo "Renaming repository code-analyze-mcp -> aptu-coder ..."
gh api repos/clouatre-labs/code-analyze-mcp \
  --method PATCH \
  --field name=aptu-coder
echo "Done. Update your remote: git remote set-url origin https://github.com/clouatre-labs/aptu-coder.git"

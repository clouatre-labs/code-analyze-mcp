#!/usr/bin/env bash
# Finalize the project rename from code-analyze-mcp to aptu-coder.
# Idempotent: each step no-ops if already completed.
set -euo pipefail

step1() {
  echo "==> Step 1: rename GitHub repository"
  if gh api repos/clouatre-labs/aptu-coder --silent 2>/dev/null; then
    echo "    Repository already renamed to aptu-coder -- skipping"
    return 0
  fi
  echo "    Renaming code-analyze-mcp -> aptu-coder ..."
  gh api repos/clouatre-labs/code-analyze-mcp \
    --method PATCH \
    --field name=aptu-coder
  echo "    Done. Update your remote: git remote set-url origin https://github.com/clouatre-labs/aptu-coder.git"
}

step2() {
  echo "==> Step 2: publish aptu-coder-core and aptu-coder to crates.io via tag push"
  local version
  version=$(grep '^version' "$(git rev-parse --show-toplevel)/Cargo.toml" | head -1 | sed 's/.*= *"\(.*\)"/\1/')
  local tag="v${version}"

  # Idempotency: check local tag first, then origin
  if git rev-parse -q --verify "refs/tags/$tag" > /dev/null; then
    echo "    Local tag $tag already exists -- reusing it for push"
  else
    echo "    Creating GPG-signed annotated tag $tag..."
    git tag -s "$tag" -m "chore(release): $tag"
  fi
  if git ls-remote --tags origin "$tag" | grep -q "$tag"; then
    echo "    Tag $tag already exists on origin -- skipping push"
  else
    git push origin "$tag"
    echo "    Tag pushed. release.yml will publish aptu-coder-core and aptu-coder."
  fi

  echo "    Polling crates.io for aptu-coder v${version} (up to 30 min)..."
  local attempts=0
  local max=180
  while [ $attempts -lt $max ]; do
    local status
    status=$(curl -s -o /dev/null -w "%{http_code}" \
      -H "User-Agent: rename-finalize/1.0 (clouatre-labs/aptu-coder)" \
      "https://crates.io/api/v1/crates/aptu-coder/${version}")
    if [ "$status" = "200" ]; then
      echo "    aptu-coder v${version} is live on crates.io."
      return 0
    fi
    attempts=$((attempts + 1))
    echo "    Attempt $attempts/$max: not yet available (HTTP $status). Waiting 10s..."
    sleep 10
  done
  echo "ERROR: aptu-coder v${version} did not appear on crates.io within 30 minutes." >&2
  echo "       Check release.yml workflow run on GitHub Actions." >&2
  return 1
}

step2b() {
  echo "==> Step 2b: publish tombstone patch for old crate names"
  : "${CARGO_REGISTRY_TOKEN:?ERROR: CARGO_REGISTRY_TOKEN must be set}"
  export CARGO_REGISTRY_TOKEN

  local tombstone_version="0.6.1"
  local repo_url="https://github.com/clouatre-labs/aptu-coder"

  for crate in code-analyze-mcp code-analyze-core; do
    echo "    Processing tombstone for ${crate}..."
    local tmpdir
    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' RETURN

    mkdir -p "$tmpdir/src"
    cat > "$tmpdir/Cargo.toml" << TOML
[package]
name = "${crate}"
version = "${tombstone_version}"
edition = "2021"
description = "DEPRECATED: this crate has been renamed to aptu-coder. Please update your dependency to aptu-coder. See ${repo_url}"
license = "Apache-2.0"
repository = "${repo_url}"
publish = true
TOML
    touch "$tmpdir/src/lib.rs"

    # Idempotency: check if tombstone version already published
    local status
    status=$(curl -s -o /dev/null -w "%{http_code}" \
      -H "User-Agent: rename-finalize/1.0 (clouatre-labs/aptu-coder)" \
      "https://crates.io/api/v1/crates/${crate}/${tombstone_version}")
    if [ "$status" = "200" ]; then
      echo "    ${crate} v${tombstone_version} already published -- skipping publish"
    else
      echo "    Publishing tombstone ${crate} v${tombstone_version}..."
      (cd "$tmpdir" && cargo publish --allow-dirty)
    fi

    echo "    Yanking ${crate} v${tombstone_version}..."
    cargo yank --version "$tombstone_version" "$crate" || true
    echo "    Done: ${crate} tombstone complete."
    rm -rf "$tmpdir"
    trap - RETURN
  done
}

step3() {
  echo "==> Step 3: yank all existing versions of old crate names"
  : "${CARGO_REGISTRY_TOKEN:?ERROR: CARGO_REGISTRY_TOKEN must be set}"
  export CARGO_REGISTRY_TOKEN

  for crate in code-analyze-mcp code-analyze-core; do
    echo "    Fetching versions for ${crate}..."
    local response
    response=$(curl -s \
      -H "User-Agent: rename-finalize/1.0 (clouatre-labs/aptu-coder)" \
      "https://crates.io/api/v1/crates/${crate}")

    if echo "$response" | jq -e '.errors' > /dev/null 2>&1; then
      echo "    ${crate} not found on crates.io -- skipping"
      continue
    fi

    local versions
    versions=$(echo "$response" | jq -r '.versions[].num')
    if [ -z "$versions" ]; then
      echo "    No versions found for ${crate} -- skipping"
      continue
    fi

    echo "$versions" | while read -r ver; do
      echo "    Yanking ${crate} v${ver}..."
      cargo yank --version "$ver" "$crate" || echo "    (already yanked or error -- continuing)"
      sleep 1
    done
    echo "    Done: all versions of ${crate} yanked."
  done
}

step4() {
  echo "==> Step 4: verify Homebrew tap PR"
  local pr_url
  pr_url=$(gh pr list \
    --repo clouatre-labs/homebrew-tap \
    --search "aptu-coder" \
    --json url,title \
    --jq '.[0].url // ""')

  if [ -n "$pr_url" ]; then
    echo "    Homebrew tap PR found: $pr_url"
    echo "    Review and merge it at the URL above."
  else
    echo "    No Homebrew tap PR found yet."
    echo "    The release.yml workflow opens it automatically after publishing."
    echo "    Check: https://github.com/clouatre-labs/homebrew-tap/pulls"
  fi
}

main() {
  step1

  step2

  echo ""
  echo "Step 2 complete. About to publish tombstone patches for old crate names (irreversible)."
  read -r -p "Continue with step 2b? [y/N] " confirm
  if [[ "$confirm" =~ ^[Yy]$ ]]; then
    step2b
  fi

  echo ""
  echo "About to yank all existing versions of old crates (irreversible)."
  read -r -p "Continue with step 3? [y/N] " confirm
  if [[ "$confirm" =~ ^[Yy]$ ]]; then
    step3
  fi

  step4
}

# Execute main if script is run directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  main
fi

#!/usr/bin/env bash
# Sync the local Warp fork with upstream and rebuild.
#
# Branch model: `master` mirrors upstream master; all local patches live on
# `local`. This script fetches upstream, fast-forwards master, rebases `local`
# on top, and rebuilds warp-oss. On rebase conflict it stops and leaves the
# rebase in progress for manual resolution (or `git rebase --abort`).
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$REPO"

if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "ERROR: uncommitted changes on $(git branch --show-current); commit or stash first." >&2
    exit 1
fi

echo "==> Fetching upstream"
git fetch origin master

echo "==> Updating master mirror"
git branch -f master origin/master

echo "==> Rebasing 'local' onto master"
git checkout local
if ! git rebase master; then
    cat >&2 << 'EOF'
==> REBASE CONFLICT. Resolve manually, then: git rebase --continue
    (or back out with: git rebase --abort)
    Likely hotspots: crates/markdown_parser (math parsing),
    crates/warpui/build.rs + prebuilt/ (Metal bypass — if upstream changed
    shaders.metal, a fresh shaders.metallib must be carved from a newer
    official Warp build), app/src/settings_view (commercial strip patches).
EOF
    exit 1
fi

echo "==> Rebuilding warp-oss"
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --bin warp-oss

echo "==> Sync complete: $(git log --oneline -1 master) + $(git rev-list master..local --count) local patches"

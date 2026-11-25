#!/usr/bin/env bash
# Materialise a minimal checkout of zed-industries/zed at the revision this
# research is pinned to, so the harness can build against the real crates.
#
# Usage: tools/setup-zed.sh [target-dir]
# Default target-dir: vendor/zed (relative to the repository root)
set -euo pipefail

ZED_PIN="2664a0aa1eae5dabf8f8273c03f88d53061ca7dc"
ZED_REMOTE="https://github.com/zed-industries/zed.git"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${1:-$REPO_ROOT/vendor/zed}"

# Cone-mode sparse checkout, filtered fetch: only the crates the harness
# actually reaches. path/util/util_macros are pulled in by text's
# test-support feature, which is what exposes check_invariants().
SPARSE_DIRS=(
  crates/text
  crates/rope
  crates/clock
  crates/sum_tree
  crates/collections
  crates/gpui_util
  crates/ztracing
  crates/ztracing_macro
  crates/util
  crates/util_macros
  crates/zlog
  crates/path
  tooling/perf
)

if [ -d "$TARGET/.git" ]; then
  current="$(git -C "$TARGET" rev-parse HEAD 2>/dev/null || true)"
  if [ "$current" = "$ZED_PIN" ]; then
    echo "zed checkout already at $ZED_PIN: $TARGET"
    exit 0
  fi
  echo "existing checkout at $TARGET is not pinned ($current), reusing it anyway"
  exit 0
fi

echo "fetching zed @ $ZED_PIN into $TARGET"
mkdir -p "$TARGET"
git -C "$TARGET" init -q
git -C "$TARGET" remote add origin "$ZED_REMOTE"
git -C "$TARGET" config core.sparseCheckout true
git -C "$TARGET" sparse-checkout init --cone
git -C "$TARGET" sparse-checkout set "${SPARSE_DIRS[@]}"
git -C "$TARGET" fetch --filter=blob:none --depth 1 origin "$ZED_PIN"
git -C "$TARGET" checkout -q FETCH_HEAD

actual="$(git -C "$TARGET" rev-parse HEAD)"
if [ "$actual" != "$ZED_PIN" ]; then
  echo "ERROR: pinned $ZED_PIN but checkout is at $actual" >&2
  exit 1
fi

echo "zed checkout ready at $actual"
echo "crates present: $(ls "$TARGET/crates" | tr '\n' ' ')"

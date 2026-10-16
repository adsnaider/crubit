#!/bin/bash
# Part of the Crubit project, under the Apache License v2.0 with LLVM
# Exceptions. See /LICENSE for license information.
# SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

# Walks through the crubit + zngur demo (see README.md), one step at a time.
#
# Usage, from the root of the crubit repository:
#
#     examples/zngur/demo/demo.sh             # Pauses between steps.
#     examples/zngur/demo/demo.sh --no-pause  # Doesn't.

set -euo pipefail

cd "$(dirname "$0")/../../.."
readonly PKG=examples/zngur/demo
BAZEL="${BAZEL:-bazelisk}"
PAUSE=1
if [[ "${1:-}" == "--no-pause" ]]; then
  PAUSE=0
fi

bold() { printf '\n\033[1m%s\033[0m\n' "$*"; }
step() {
  if (( PAUSE )); then
    read -r -p $'\n[press Enter for the next step] ' _
  fi
  printf '\n\033[1;34m==== %s ====\033[0m\n' "$*"
}
build() { "$BAZEL" build --ui_event_filters=-info,-debug,-stdout,-stderr --noshow_progress "$@"; }
bin() { "$BAZEL" info bazel-bin 2>/dev/null; }
# The `// skipped:` lines of a `.zng` file, shortened.
skipped() { grep '^// skipped:' "$1" | cut -c 1-150; }
# The public Rust functions and types of zngur's generated Rust code.
rust_api() {
  grep -oE 'pub (unsafe )?fn [A-Za-z0-9_]+|pub use [^;]* as [A-Za-z0-9_]+|pub type [A-Za-z0-9_]+' "$1" \
    | sed -E 's/pub use .* as /fn /; s/pub (unsafe )?//' | grep -v ' _' | sort -u
}

step "1. A C++ library, and one BUILD rule"
cat "$PKG/shapes.h" | sed -n '/^namespace shapes/,/^}  \/\/ namespace shapes/p'
bold "BUILD:"
sed -n '/name = "shapes_generated"/,/^)/p' "$PKG/BUILD" | sed '1s/^/cc_zngur_bindings(\n/'

step "2. What crubit generates: a .zng file (the API, and what it skipped)"
build "//$PKG:shapes_generated_zng"
readonly GENERATED="$(bin)/$PKG/shapes_generated.zng"
bold "Skipped items:"
skipped "$GENERATED"
bold "The API ($(grep -cv '^//' "$GENERATED") lines; zngur generates all the glue from it):"
grep -v '^//' "$GENERATED"

step "3. Add the skipped items back by hand: shapes_extra.zng"
cat "$PKG/shapes_extra.zng"
bold "BUILD:"
sed -n '/name = "shapes_zngur"/,/^)/p' "$PKG/BUILD" | sed '1s/^/cc_zngur_bindings(\n/'

step "4. What Rust gets now, that it didn't before"
build "//$PKG:shapes_generated_rs" "//$PKG:shapes_zngur_rs"
diff <(rust_api "$(bin)/$PKG/shapes_generated.zng.rs") \
  <(rust_api "$(bin)/$PKG/shapes_zngur.zng.rs") | grep '^>' | sed 's/^>/  +/' || true

step "5. Rust calling C++ (main.rs)"
sed -n '/^fn main/,$p' "$PKG/main.rs"
bold "bazel run //$PKG:main"
"$BAZEL" run --ui_event_filters=-info,-debug --noshow_progress "//$PKG:main"

step "6. The other direction: C++ calling a Rust crate (stats.rs)"
build "//$PKG:stats_generated_zng"
bold "Skipped items in stats_generated.zng:"
skipped "$(bin)/$PKG/stats_generated.zng"
bold "Added back by hand (stats_extra.zng):"
grep -v '^//' "$PKG/stats_extra.zng" | cat -s
bold "bazel run //$PKG:main_cc"
"$BAZEL" run --ui_event_filters=-info,-debug --noshow_progress "//$PKG:main_cc"

step "7. Compared with crubit's own bindings"
cat "$PKG/comparison.md"

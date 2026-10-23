#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Vendors Vitrine's browser packages, @vitrine/verify and @vitrine/ui, as npm
# tarballs in packages/vendor-npm/, from a Vitrine checkout that it leaves
# untouched:
#
#   scripts/vendor-vitrine.sh <vitrine checkout> [<ref>]    (ref: main)
#
# It exports the ref's packages/ and testdata/ with `git archive` into a
# temporary directory, installs Vitrine's locked dependencies there, builds and
# tests both packages, and packs them. The tarballs keep fixed names, so the
# apps' `file:` dependencies don't change with Vitrine's version. It drops the
# packages' entries from packages/yarn.lock, whose hashes no longer match.
#
# Afterwards, run `yarn install` in packages/ so that yarn.lock records the
# new tarballs, and commit the tarballs, VITRINE_COMMIT and yarn.lock.

set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "usage: $0 <vitrine checkout> [<ref>]" >&2
    exit 2
fi

VITRINE="$(realpath "$1")"
REF="${2:-main}"
SCRIPT_DIR="$(cd -- "$(dirname "$0")" >/dev/null 2>&1 && pwd -P)"
PACKAGES_DIR="$(realpath "$SCRIPT_DIR/../packages")"
VENDOR_DIR="$PACKAGES_DIR/vendor-npm"
COMMIT="$(git -C "$VITRINE" rev-parse --verify "$REF^{commit}")"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

git -C "$VITRINE" archive "$COMMIT" packages testdata | tar -x -C "$WORK"
mkdir "$WORK/out"
(
    cd "$WORK/packages"
    npm ci --no-audit --no-fund
    npm run build -w @vitrine/verify -w @vitrine/ui
    npm test -w @vitrine/verify -w @vitrine/ui
    npm pack -w @vitrine/verify -w @vitrine/ui --pack-destination "$WORK/out"
)

mkdir -p "$VENDOR_DIR"
cp "$WORK"/out/vitrine-verify-*.tgz "$VENDOR_DIR/vitrine-verify.tgz"
cp "$WORK"/out/vitrine-ui-*.tgz "$VENDOR_DIR/vitrine-ui.tgz"
echo "$COMMIT" >"$VENDOR_DIR/VITRINE_COMMIT"

awk '
  /^"@vitrine\// { skip = 1 }
  skip && /^$/ { skip = 0; next }
  !skip { print }
' "$PACKAGES_DIR/yarn.lock" >"$WORK/yarn.lock"
cp "$WORK/yarn.lock" "$PACKAGES_DIR/yarn.lock"

echo "Vendored @vitrine/verify and @vitrine/ui from $COMMIT into $VENDOR_DIR."
echo "Now run 'yarn install' in packages/ to update yarn.lock."

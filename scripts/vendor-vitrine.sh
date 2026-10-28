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
# It exports the ref's packages/, testdata/ and specs/ with `git archive` into a
# temporary directory, installs Vitrine's locked dependencies there, builds and
# tests both packages, and packs them; a failing test stops it. The tarballs keep fixed
# names, so the apps' `file:` dependencies don't change with Vitrine's version.
# It drops the packages' entries from packages/yarn.lock, whose hashes no
# longer match, and yarn's unpacked copies of the old tarballs.
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

git -C "$VITRINE" archive "$COMMIT" packages testdata specs | tar -x -C "$WORK"
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
(cd "$VENDOR_DIR" && sha256sum vitrine-verify.tgz vitrine-ui.tgz > SHA256SUMS)

awk '
  /^"@vitrine\// { skip = 1 }
  skip && /^$/ { skip = 0; next }
  !skip { print }
' "$PACKAGES_DIR/yarn.lock" >"$WORK/yarn.lock"
cp "$WORK/yarn.lock" "$PACKAGES_DIR/yarn.lock"

# Yarn 1 keeps an unpacked copy of each file: tarball in its cache's .tmp,
# keyed by the tarball's path, and installs that copy again while it exists,
# so it would never see the new tarballs. Drop the copies of these two.
YARN_CACHE="$(cd "$PACKAGES_DIR" && yarn cache dir 2>/dev/null || true)"
if [[ -n "$YARN_CACHE" && -d "$YARN_CACHE/.tmp" ]]; then
    for copy in "$YARN_CACHE"/.tmp/*/; do
        if grep -qE '"name": "@vitrine/(verify|ui)"' "$copy/package.json" 2>/dev/null; then
            rm -rf "$copy"
        fi
    done
fi
rm -rf "$PACKAGES_DIR/node_modules/@vitrine"

echo "Vendored @vitrine/verify and @vitrine/ui from $COMMIT into $VENDOR_DIR."
echo "Now run 'yarn install' in packages/ to update yarn.lock."

#!/usr/bin/env bash
set -euo pipefail

# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Full rebuild of sequent-core WASM + JS frontend packages.
#
# Run from inside packages/sequent-core via nix develop:
#   cd packages/sequent-core && nix develop --command ../../.devcontainer/scripts/rebuild-sequent-core-full.sh

SEQUENT_CORE_DIR="$(pwd)"
PACKAGES_DIR="$(dirname "$SEQUENT_CORE_DIR")"

echo "==> Checking versions..."
rustc --version
wasm-pack --version

echo "==> Building sequent-core WASM..."
wasm-pack build --mode no-install --out-name index --release --target web \
    --features=wasmtest

echo "==> Packing..."
wasm-pack -v pack . 2>&1 | tee output.log

echo "==> Updating yarn.lock..."
cd "$PACKAGES_DIR"
hash=$(grep "shasum:" sequent-core/output.log | awk '{printf $4}')
hash="${hash}\\\""
awk -v hash="${hash}" '
  /sequent-core-0.1.0.tgz#/ {
    sub(/#.*/, "#"hash"")
  }
  { print }
' yarn.lock > yarn.lock.tmp
mv yarn.lock.tmp yarn.lock
rm sequent-core/output.log

echo "==> Clearing yarn cache and replacing tgz files..."
rm -Rf /home/vscode/.cache/yarn/
rm -f ./ui-core/rust/sequent-core-0.1.0.tgz \
      ./admin-portal/rust/sequent-core-0.1.0.tgz \
      ./voting-portal/rust/sequent-core-0.1.0.tgz \
      ./ballot-verifier/rust/sequent-core-0.1.0.tgz
cp sequent-core/pkg/sequent-core-0.1.0.tgz ./ui-core/rust/sequent-core-0.1.0.tgz
cp sequent-core/pkg/sequent-core-0.1.0.tgz ./admin-portal/rust/sequent-core-0.1.0.tgz
cp sequent-core/pkg/sequent-core-0.1.0.tgz ./voting-portal/rust/sequent-core-0.1.0.tgz
cp sequent-core/pkg/sequent-core-0.1.0.tgz ./ballot-verifier/rust/sequent-core-0.1.0.tgz

echo "==> Cleaning node_modules and dist..."
rm -rf node_modules */node_modules dist */dist

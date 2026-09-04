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
    --features=wasmtest,default_features

echo "==> Placing the package where the workspace packages expect it..."
cd "$PACKAGES_DIR"
rm -Rf /home/vscode/.cache/yarn/
for dir in ui-core admin-portal voting-portal ballot-verifier; do
    rm -rf "./${dir}/rust/pkg"
    mkdir -p "./${dir}/rust"
    cp -a sequent-core/pkg "./${dir}/rust/pkg"
done

echo "==> Cleaning node_modules and dist..."
rm -rf node_modules */node_modules dist */dist

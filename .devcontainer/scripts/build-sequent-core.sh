#!/usr/bin/env bash
set -euo pipefail

# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Note: NIX_HARDENING_ENABLE and CFLAGS are now configured in flake.nix shellHook
# wasm-bindgen-cli is pinned to a version in flake.nix to match Cargo.toml

TARGET_DIR=/workspaces/step/packages/sequent-core
cd "$TARGET_DIR"
which rustc
rustc --version
which cargo
cargo --version
which wasm-pack
wasm-pack --version
which wasm-bindgen
wasm-bindgen --version

wasm-pack build --mode no-install --out-name index --release --target web --features=wasmtest,default_features

# The package is a dependency of the workspace packages as a directory (file:./rust/pkg),
# not as a committed tarball, so it is copied into place and yarn.lock never changes.
cd ..
for dir in ui-core admin-portal voting-portal ballot-verifier; do
    rm -rf "./${dir}/rust/pkg"
    mkdir -p "./${dir}/rust"
    cp -a sequent-core/pkg "./${dir}/rust/pkg"
done

rm -rf node_modules ui-core/node_modules voting-portal/node_modules ballot-verifier/node_modules admin-portal/node_modules

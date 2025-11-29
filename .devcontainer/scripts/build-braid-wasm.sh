#!/usr/bin/env bash
set -euo pipefail

# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Match sequent-core build environment tweaks for wasm/ring
export NIX_HARDENING_ENABLE=""
# If CFLAGS_wasm32_unknown_unknown is already provided by the dev shell (with
# proper clang resource/include paths), do not overwrite it; otherwise, fall
# back to a minimal optimisation-only setting.
: "${CFLAGS_wasm32_unknown_unknown:=-O3 -ffunction-sections -fdata-sections -fno-exceptions}"
export CFLAGS_wasm32_unknown_unknown

TARGET_DIR=/workspaces/step/packages/braid-wasm
cd "$TARGET_DIR"
which rustc
rustc --version
which cargo
cargo --version
which wasm-pack
wasm-pack --version
which wasm-bindgen
wasm-bindgen --version

wasm-pack build --mode no-install --out-name index --release --target web --features=wasmtest
wasm-pack -v pack . 2>&1 | tee output.log

cd ..
hash=$(grep "shasum:" braid-wasm/output.log | awk '{printf $4}')
hash="${hash}\\\""
awk -v hash="${hash}" '
  /braid-wasm-0.1.0.tgz#/ {
    sub(/#.*/, "#"hash"")
  }
  { print }
' yarn.lock > yarn.lock.tmp

mv yarn.lock.tmp yarn.lock

rm braid-wasm/output.log

# Copy the freshly built npm package into admin-portal rust deps
mkdir -p ./admin-portal/rust
rm -f ./admin-portal/rust/braid-wasm-0.1.0.tgz
cp braid-wasm/pkg/braid-wasm-0.1.0.tgz ./admin-portal/rust/braid-wasm-0.1.0.tgz

# Force dependent app(s) to reinstall node_modules on next yarn install
rm -rf admin-portal/node_modules

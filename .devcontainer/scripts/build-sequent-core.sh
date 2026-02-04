#!/usr/bin/env bash
set -euo pipefail

# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# This is to resolve an issue where nix -fzero-call-used-regs=used-gpr hardening
# flag, which is not supported when compiling C code for WebAssembly targets
# (used by the ring cryptographic library):
export NIX_HARDENING_ENABLE=""
export CFLAGS_wasm32_unknown_unknown="${CFLAGS_wasm32_unknown_unknown:-} -O3 -ffunction-sections -fdata-sections -fno-exceptions"

# Pin wasm-bindgen-cli version to match the wasm-bindgen crate in Cargo.toml
WASM_BINDGEN_VERSION="0.2.100"

TARGET_DIR=/workspaces/step/packages/sequent-core
cd "$TARGET_DIR"
which rustc
rustc --version
which cargo
cargo --version
which wasm-pack
wasm-pack --version

# Install exact wasm-bindgen-cli version if not already installed or version mismatch
INSTALLED_VERSION=$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || echo "none")
if [ "$INSTALLED_VERSION" != "$WASM_BINDGEN_VERSION" ]; then
    echo "Installing wasm-bindgen-cli $WASM_BINDGEN_VERSION (current: $INSTALLED_VERSION)"
    cargo install wasm-bindgen-cli --version "$WASM_BINDGEN_VERSION" --force
fi
which wasm-bindgen
wasm-bindgen --version

wasm-pack build --mode no-install --out-name index --release --target web --features=wasmtest,default_features
wasm-pack -v pack . 2>&1 | tee output.log

cd ..
hash=$(grep "shasum:" sequent-core/output.log | awk '{printf $4}')
awk -v hash="${hash}" '
  /^"sequent-core@file:/ { in_sequent = 1 }
  /^"[^"]+":$/ && !/^"sequent-core@file:/ { in_sequent = 0 }
  /sequent-core-0.1.0.tgz#/ {
    sub(/#.*/, "#"hash"\"")
  }
  /^  uid "/ && in_sequent {
    sub(/"[^"]*"$/, "\""hash"\"")
  }
  { print }
' yarn.lock > yarn.lock.tmp

mv yarn.lock.tmp yarn.lock

rm sequent-core/output.log
rm ./ui-core/rust/sequent-core-0.1.0.tgz ./admin-portal/rust/sequent-core-0.1.0.tgz ./voting-portal/rust/sequent-core-0.1.0.tgz ./ballot-verifier/rust/sequent-core-0.1.0.tgz
cp sequent-core/pkg/sequent-core-0.1.0.tgz ./ui-core/rust/sequent-core-0.1.0.tgz
cp sequent-core/pkg/sequent-core-0.1.0.tgz ./admin-portal/rust/sequent-core-0.1.0.tgz
cp sequent-core/pkg/sequent-core-0.1.0.tgz ./voting-portal/rust/sequent-core-0.1.0.tgz
cp sequent-core/pkg/sequent-core-0.1.0.tgz ./ballot-verifier/rust/sequent-core-0.1.0.tgz

rm -rf node_modules ui-core/node_modules voting-portal/node_modules ballot-verifier/node_modules admin-portal/node_modules

# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

#!/usr/bin/env bash
# set -euo pipefail

# Colors
GREEN="\e[32m"
CYAN="\e[36m"
RED="\e[31m"
RESET="\e[0m"

# Enter crate directory
cd "$(dirname "$0")/.."

echo -e "${GREEN}Building WASM with atomics support...${RESET}"
echo -e "${CYAN}Using stable toolchain from flake.nix (RUSTC_BOOTSTRAP=1 unlocks -Z build-std)...${RESET}"

# Run the whole cargo build + wasm-bindgen + npm pack pipeline inside a single,
# non-nested `nix develop` shell. Running `nix develop` nested inside an
# already-active devenv shell causes cargo's build-script-build binaries to
# segfault at process startup (non-deterministically, a different crate each
# run) - this reproduces 100% of the time nested, and never standalone, so
# braid/flake.nix's devShell is made self-sufficient (cargo, wasm-bindgen, npm)
# to avoid the nesting entirely.
INNER_SCRIPT=$(mktemp)
trap 'rm -f "$INNER_SCRIPT"' EXIT
cat > "$INNER_SCRIPT" <<'INNER_EOF'
set -e

# -Z build-std requires nightly Cargo, so RUSTC_BOOTSTRAP=1 is used to permit
# it on the stable toolchain.
export RUSTC_BOOTSTRAP=1

# The atomics/bulk-memory/mutable-globals flags must be scoped to the wasm32
# target only (not the global RUSTFLAGS) - otherwise Cargo also applies them
# to native host build-script/proc-macro compilation, which produces
# corrupted aarch64 codegen and segfaults at process startup.
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals"

echo "Compiling to WASM..."
cargo build --lib --target wasm32-unknown-unknown --release --no-default-features --features wasm -Z build-std=panic_abort,std --target-dir target

TARGET_DIR="target"
WASM_FILE="${TARGET_DIR}/wasm32-unknown-unknown/release/braid.wasm"
echo "Generating JS bindings with wasm-bindgen..."
echo "Using WASM file: ${WASM_FILE}"
wasm-bindgen "${WASM_FILE}" --out-dir pkg --target web

echo "Packaging with npm pack..."
# Leading "./" forces npm to treat this as a local path, not a registry
# package specifier (a real npm package is named "pkg" - without "./",
# `npm pack` fetches that instead of packing this local directory).
PKG_DIR="./pkg"
ADMIN_PORTAL_RUST="../admin-portal/rust"

# Create package.json for npm
cat > "${PKG_DIR}/package.json" << 'PKG_EOF'
{
  "name": "braid-wasm",
  "type": "module",
  "version": "0.1.0",
  "files": [
    "*.js",
    "*.wasm",
    "*.d.ts",
    "LICENSE*",
    "snippets/"
  ],
  "main": "braid.js",
  "types": "braid.d.ts",
  "sideEffects": [
    "./snippets/*"
  ]
}
PKG_EOF

# Use npm pack to create the tarball and capture output for hash extraction
npm pack "${PKG_DIR}" 2>&1 | tee output.log

SHASUM=$(grep "shasum:" output.log | awk '{print $4}')
echo "Package shasum: ${SHASUM}"

# Move the tarball to admin-portal/rust
mv braid-wasm-0.1.0.tgz "${ADMIN_PORTAL_RUST}/"
echo "Copied braid-wasm-0.1.0.tgz to admin-portal/rust/"
INNER_EOF

if ! nix develop --command bash "$INNER_SCRIPT"; then
    echo -e "${RED}Build failed!${RESET}"
    exit 1
fi

# Re-read the shasum from output.log (written by the inner script, in the
# current directory) since the inner script ran in a separate process.
SHASUM=$(grep "shasum:" output.log | awk '{print $4}')

# Update yarn.lock with new hash
cd ..
echo -e "${CYAN}Updating yarn.lock hash...${RESET}"
SHASUM_ESCAPED="${SHASUM}\""
awk -v hash="${SHASUM_ESCAPED}" '
  /braid-wasm-0.1.0.tgz#/ {
    sub(/#.*/, "#"hash"")
  }
  { print }
' yarn.lock > yarn.lock.tmp
mv yarn.lock.tmp yarn.lock

# Clean up
rm -f braid/output.log

echo -e "${GREEN}Build complete! WASM bundle ready in packages/braid/pkg/${RESET}"
echo -e "${GREEN}Admin portal dependency updated in packages/admin-portal/rust/${RESET}"

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
cd ../braid

echo -e "${GREEN}Building WASM with atomics support...${RESET}"

# Use nightly toolchain from flake.nix
echo -e "${CYAN}Using nightly toolchain from flake.nix...${RESET}"

# Build with nightly and atomics support
echo -e "${CYAN}Compiling to WASM...${RESET}"
if ! nix develop --command bash -c 'export RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals" && cargo build --lib --target wasm32-unknown-unknown --release --no-default-features --features wasm -Z build-std=panic_abort,std --target-dir target'; then
    echo -e "${RED}Cargo build failed!${RESET}"
    cd ../..
    exit 1
fi

# Run wasm-bindgen
# Use local target directory (matching --target-dir in cargo build)
TARGET_DIR="target"
WASM_FILE="${TARGET_DIR}/wasm32-unknown-unknown/release/braid.wasm"
echo -e "${CYAN}Generating JS bindings with wasm-bindgen...${RESET}"
echo -e "${CYAN}Using WASM file: ${WASM_FILE}${RESET}"

# Use wasm-bindgen - requires devenv shell or wasm-bindgen in PATH
if ! command -v wasm-bindgen &> /dev/null; then
    echo -e "${RED}wasm-bindgen not found. Please run this script from devenv shell.${RESET}"
    cd ../..
    exit 1
fi

if ! wasm-bindgen "${WASM_FILE}" --out-dir ../wbraid/pkg --target web; then
    echo -e "${RED}wasm-bindgen failed!${RESET}"
    cd ../..
    exit 1
fi

# Package using npm pack
echo -e "${CYAN}Packaging with npm pack...${RESET}"
PKG_DIR="../wbraid/pkg"
ADMIN_PORTAL_RUST="../admin-portal/rust"

# Create package.json for npm
cat > "${PKG_DIR}/package.json" << 'EOF'
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
EOF

# Use npm pack to create the tarball and capture output for hash extraction
npm pack "${PKG_DIR}" 2>&1 | tee output.log

# Extract shasum from npm pack output
SHASUM=$(grep "shasum:" output.log | awk '{print $4}')
echo -e "${CYAN}Package shasum: ${SHASUM}${RESET}"

# Move the tarball to admin-portal/rust
mv braid-wasm-0.1.0.tgz "${ADMIN_PORTAL_RUST}/"
echo -e "${CYAN}Copied braid-wasm-0.1.0.tgz to admin-portal/rust/${RESET}"

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

cd ..

echo -e "${GREEN}Build complete! WASM bundle ready in packages/wbraid/pkg/${RESET}"
echo -e "${GREEN}Admin portal dependency updated in packages/admin-portal/rust/${RESET}"

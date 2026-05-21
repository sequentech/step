# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

#!/usr/bin/env bash
set -euo pipefail

# Always run from the braid package directory regardless of where the script is invoked from
cd "$(dirname "$0")"

# Colors
GREEN="\e[32m"
CYAN="\e[36m"
RED="\e[31m"
RESET="\e[0m"

PKG_DIR="./pkg"
ADMIN_PORTAL_RUST="../admin-portal/rust"

# Detect current system for the nix flake devShell attribute
SYSTEM=$(nix eval --impure --raw --expr 'builtins.currentSystem')
NIX_SHELL="nix develop .#devShell.${SYSTEM} --command"

echo -e "${GREEN}Building WASM with atomics support...${RESET}"
echo -e "${CYAN}Using nightly toolchain from flake.nix (system: ${SYSTEM})...${RESET}"

# Create pkg dir and package.json before entering nix shell
mkdir -p "${PKG_DIR}"
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

# Run cargo build, wasm-bindgen, and npm pack in a single nix develop invocation
if ! ${NIX_SHELL} bash -c "
  set -euo pipefail

  echo -e '${CYAN}Compiling to WASM...${RESET}'
  export RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals'
  cargo build --lib --target wasm32-unknown-unknown --release --no-default-features --features wasm -Z build-std=panic_abort,std --target-dir target/wasm

  echo -e '${CYAN}Generating JS bindings with wasm-bindgen...${RESET}'
  wasm-bindgen target/wasm/wasm32-unknown-unknown/release/braid.wasm --out-dir ${PKG_DIR} --target web

  echo -e '${CYAN}Packaging with npm pack...${RESET}'
  npm pack ${PKG_DIR} 2>&1 | tee output.log
"; then
    echo -e "${RED}Build failed!${RESET}"
    exit 1
fi

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

echo -e "${GREEN}Build complete! WASM bundle ready in packages/braid/pkg/${RESET}"
echo -e "${GREEN}Admin portal dependency updated in packages/admin-portal/rust/${RESET}"

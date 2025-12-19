# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

#!/usr/bin/env bash
set -euo pipefail

# Colors
GREEN="\e[32m"
CYAN="\e[36m"
RED="\e[31m"
RESET="\e[0m"

# Enter crate directory
cd crates/braid-wasm

echo -e "${GREEN}Building WASM with atomics support...${RESET}"

echo -e "${CYAN}Setting nightly toolchain override...${RESET}"
rustup override set nightly >/dev/null 2>&1

# Build with nightly
echo -e "${CYAN}Compiling to WASM...${RESET}"
if ! cargo +nightly build --target wasm32-unknown-unknown --release; then
    echo -e "${RED}Cargo build failed!${RESET}"
    rustup override unset >/dev/null 2>&1
    cd ../..
    exit 1
fi

# Run wasm-bindgen
echo -e "${CYAN}Generating JS bindings with wasm-bindgen...${RESET}"
if ! wasm-bindgen ../../target/wasm32-unknown-unknown/release/braid_wasm.wasm \
        --out-dir pkg \
        --target web; then
    echo -e "${RED}wasm-bindgen failed!${RESET}"
    rustup override unset >/dev/null 2>&1
    cd ../..
    exit 1
fi

echo -e "${CYAN}Removing toolchain override...${RESET}"
rustup override unset >/dev/null 2>&1

cd ../..

echo -e "${GREEN}Build complete! WASM bundle ready in crates/braid-wasm/pkg/${RESET}"

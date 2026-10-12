#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Build WASM client with atomics support for wasm-bindgen-rayon.
#
# Bash twin of build-wasm.ps1 for the devcontainer, modeled on
# packages/braid/scripts/build-wasm.sh: no nightly toolchain override —
# RUSTC_BOOTSTRAP=1 lets the stable toolchain (which ships rust-src) accept
# the nightly-only bits that crates/braid/.cargo/config.toml asks for
# ([unstable] build-std and the atomics/shared-memory rustflags).
# Based on: https://github.com/huggingface/xet-core/issues/554

set -euo pipefail

GREEN=$'\e[32m'
CYAN=$'\e[36m'
RED=$'\e[31m'
RESET=$'\e[0m'

cd "$(dirname "$0")"
ROOT=$(pwd)

# cargo resolves a relative CARGO_TARGET_DIR against the cwd, and the
# devcontainer sets a relative one (rust-local-target); anchor it here before
# cd-ing into crates/braid so this build shares the workspace's target dir.
case "${CARGO_TARGET_DIR:-}" in
    "" | /*) ;;
    *) export CARGO_TARGET_DIR="$ROOT/$CARGO_TARGET_DIR" ;;
esac

# A RUSTFLAGS in the environment overrides the [target.wasm32-unknown-unknown]
# rustflags in crates/braid/.cargo/config.toml entirely (env beats config), and
# the devcontainer's devenv exports RUSTFLAGS=-Awarnings — which silently drops
# the atomics flags and fails the build inside wasm-bindgen-rayon.
unset RUSTFLAGS

# wasm-bindgen-cli must match the wasm-bindgen crate version exactly (the
# devcontainer's devenv.nix pins the CLI to it); check before spending minutes
# on a build whose bindings can't be generated.
WANT=$(sed -n '/^name = "wasm-bindgen"$/{n;s/^version = "\(.*\)"$/\1/p}' Cargo.lock)
HAVE=$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)
if [ "$HAVE" != "$WANT" ]; then
    echo "${RED}wasm-bindgen-cli ${HAVE:-not found} on PATH, but this workspace pins wasm-bindgen ${WANT}.${RESET}"
    echo "Install the matching CLI and put it first on PATH, e.g.:"
    echo "  cargo install wasm-bindgen-cli --version ${WANT} --locked --root ~/.wbraid-tools"
    echo "  export PATH=~/.wbraid-tools/bin:\$PATH"
    exit 1
fi

echo "${GREEN}Building WASM with atomics support...${RESET}"

cd crates/braid

# Build using cargo (picks up .cargo/config.toml with atomics + linker flags)
echo "${CYAN}Compiling to WASM (stable toolchain, RUSTC_BOOTSTRAP=1)...${RESET}"
if ! RUSTC_BOOTSTRAP=1 cargo build --lib --target wasm32-unknown-unknown --release --no-default-features --features wasm; then
    echo "${RED}Cargo build failed!${RESET}"
    exit 1
fi

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
WASM_FILE="$TARGET_DIR/wasm32-unknown-unknown/release/braid.wasm"

# Run wasm-bindgen to generate JS bindings
echo "${CYAN}Generating JS bindings with wasm-bindgen...${RESET}"
if ! wasm-bindgen "$WASM_FILE" --out-dir pkg --target web; then
    echo "${RED}wasm-bindgen failed!${RESET}"
    exit 1
fi

echo "${GREEN}Build complete! WASM bundle ready in crates/braid/pkg/${RESET}"

#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Headless-browser test for the wasm IndexedDB persistence backend (M3-B).
# Bash twin of test-wasm.ps1.
#
# Runs the `wasm-core` build (no `wasm-bindgen-rayon`, hence no atomics / shared
# memory), so it works in plain headless Chrome with no SharedArrayBuffer /
# COOP-COEP setup. The production browser build is unaffected: `build-wasm.sh`
# still uses `--features wasm`, which adds the `wasm-bindgen-rayon` thread pool.
#
# IMPORTANT: runs from the repo root (this directory), NOT crates/braid, so the
# atomics `.cargo/config.toml` in crates/braid is not applied.
#
# Prerequisites: `wasm-bindgen-test-runner` (ships with wasm-bindgen-cli) and a
# `chromedriver` matching your Chrome, both on PATH.

set -euo pipefail

GREEN=$'\e[32m'
RED=$'\e[31m'
RESET=$'\e[0m'

cd "$(dirname "$0")"

# The test runner must match the wasm-bindgen crate version exactly, and the
# devcontainer pins the CLI for the main workspace (a different version).
WANT=$(sed -n '/^name = "wasm-bindgen"$/{n;s/^version = "\(.*\)"$/\1/p}' Cargo.lock)
HAVE=$(wasm-bindgen-test-runner --version 2>/dev/null | awk '{print $2}' || true)
if [ "$HAVE" != "$WANT" ]; then
    echo "${RED}wasm-bindgen-test-runner ${HAVE:-not found} on PATH, but this workspace pins wasm-bindgen ${WANT}.${RESET}"
    echo "Install the matching CLI and put it first on PATH, e.g.:"
    echo "  cargo install wasm-bindgen-cli --version ${WANT} --locked --root ~/.wbraid-tools"
    echo "  export PATH=~/.wbraid-tools/bin:\$PATH"
    exit 1
fi
if ! command -v chromedriver >/dev/null 2>&1; then
    echo "${RED}chromedriver not found on PATH (needs to match your Chrome/Chromium).${RESET}"
    exit 1
fi

echo "${GREEN}Running wasm IndexedDB test in headless Chrome...${RESET}"

export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="wasm-bindgen-test-runner"
# Set NO_HEADLESS=1 in your shell to watch the browser.

if cargo test -p braid \
    --no-default-features --features wasm-core \
    --target wasm32-unknown-unknown \
    --test wasm_indexeddb; then
    echo "${GREEN}wasm IndexedDB test passed.${RESET}"
else
    code=$?
    echo "${RED}wasm IndexedDB test failed (exit $code).${RESET}"
    exit $code
fi

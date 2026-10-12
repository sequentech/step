# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Headless-browser test for the wasm IndexedDB persistence backend (M3-B).
#
# Runs the `wasm-core` build (no `wasm-bindgen-rayon`, hence no atomics / shared
# memory), so it works in plain headless Chrome with no SharedArrayBuffer /
# COOP-COEP setup. The production browser build is unaffected: `build-wasm.ps1`
# still uses `--features wasm`, which adds the `wasm-bindgen-rayon` thread pool.
#
# IMPORTANT: run from the repo root (this directory), NOT crates/braid, so the
# atomics `.cargo/config.toml` in crates/braid is not applied.
#
# Prerequisites: `wasm-bindgen-test-runner` (ships with wasm-bindgen-cli) and a
# `chromedriver` matching your Chrome, both on PATH.

Write-Host "Running wasm IndexedDB test in headless Chrome..." -ForegroundColor Green

$env:CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER = "wasm-bindgen-test-runner"
# Set NO_HEADLESS=1 in your shell to watch the browser.

cargo test -p braid `
    --no-default-features --features wasm-core `
    --target wasm32-unknown-unknown `
    --test wasm_indexeddb

$code = $LASTEXITCODE

Remove-Item Env:CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER -ErrorAction SilentlyContinue

if ($code -eq 0) {
    Write-Host "wasm IndexedDB test passed." -ForegroundColor Green
} else {
    Write-Host "wasm IndexedDB test failed (exit $code)." -ForegroundColor Red
}
exit $code

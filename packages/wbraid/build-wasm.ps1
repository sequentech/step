# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Build WASM client with atomics support for wasm-bindgen-rayon
# This requires nightly Rust and proper linker flags for SharedArrayBuffer
# Based on: https://github.com/huggingface/xet-core/issues/554

# Preflight: wasm-bindgen (ships with wasm-bindgen-cli) must match the wasm-bindgen
# crate version this workspace pins EXACTLY. The crate stamps a schema version
# into the .wasm and the CLI refuses any other; the mismatch only surfaces after
# a full compile. The pin is read from Cargo.lock so this follows future bumps.
$want = (Select-String -Path Cargo.lock -Pattern '^name = "wasm-bindgen"$' -Context 0,1 |
    Select-Object -First 1).Context.PostContext[0] -replace '^version = "(.*)"$', '$1'
$have = $null
if (Get-Command wasm-bindgen -ErrorAction SilentlyContinue) {
    $have = ((& wasm-bindgen --version) | Select-Object -First 1) -replace '^\S+\s+(\S+).*', '$1'
}
if ($have -ne $want) {
    Write-Host "wasm-bindgen $(if ($have) { $have } else { 'not found' }) on PATH, but this workspace pins wasm-bindgen $want." -ForegroundColor Red
    Write-Host "Install the matching CLI (it provides both wasm-bindgen and wasm-bindgen-test-runner):" -ForegroundColor Yellow
    Write-Host "  cargo install wasm-bindgen-cli --version $want --locked --force" -ForegroundColor Yellow
    exit 1
}

cd crates/braid

Write-Host "Building WASM with atomics support..." -ForegroundColor Green

# Set nightly toolchain override for this directory
Write-Host "Setting nightly toolchain override..." -ForegroundColor Cyan
rustup override set nightly 2>&1 | Out-Null

# Build using cargo (picks up .cargo/config.toml with atomics + linker flags)
Write-Host "Compiling to WASM..." -ForegroundColor Cyan
cargo +nightly build --lib --target wasm32-unknown-unknown --release --no-default-features --features wasm

$buildResult = $LASTEXITCODE

if ($buildResult -ne 0) {
    Write-Host "Cargo build failed!" -ForegroundColor Red
    rustup override unset 2>&1 | Out-Null
    cd ../..
    exit 1
}

# Run wasm-bindgen to generate JS bindings
Write-Host "Generating JS bindings with wasm-bindgen..." -ForegroundColor Cyan
wasm-bindgen ../../target/wasm32-unknown-unknown/release/braid.wasm --out-dir pkg --target web

$bindgenResult = $LASTEXITCODE

# Remove the override
Write-Host "Removing toolchain override..." -ForegroundColor Cyan
rustup override unset 2>&1 | Out-Null

cd ../..

if ($bindgenResult -ne 0) {
    Write-Host "wasm-bindgen failed!" -ForegroundColor Red
    exit 1
}

Write-Host "Build complete! WASM bundle ready in crates/braid/pkg/" -ForegroundColor Green

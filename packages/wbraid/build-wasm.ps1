# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Build WASM client with atomics support for wasm-bindgen-rayon
# This requires nightly Rust and proper linker flags for SharedArrayBuffer
# Based on: https://github.com/huggingface/xet-core/issues/554

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

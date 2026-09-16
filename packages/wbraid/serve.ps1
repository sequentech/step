# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Build and serve the braid WASM client with atomics support

# Clear any inherited RUSTFLAGS that might interfere with .cargo/config.toml
Remove-Item Env:\RUSTFLAGS -ErrorAction SilentlyContinue

.\build-wasm.ps1
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed, not starting server" -ForegroundColor Red
    exit 1
}

Write-Host "Starting development server on http://127.0.0.1:8080" -ForegroundColor Green
Write-Host "Open http://127.0.0.1:8080/emulator.html" -ForegroundColor Green
python server.py

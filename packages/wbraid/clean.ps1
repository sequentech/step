# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Clean up all generated files and databases

Write-Host "Cleaning up WBraid POC..." -ForegroundColor Yellow

# Remove SQLite database
if (Test-Path "crates/service/b4.db") {
    Remove-Item "crates/service/b4.db" -Force
    Write-Host "✓ Removed b4.db" -ForegroundColor Green
}

if (Test-Path "crates/service/b4.db-shm") {
    Remove-Item "crates/service/b4.db-shm" -Force
    Write-Host "✓ Removed b4.db-shm" -ForegroundColor Green
}

if (Test-Path "crates/service/b4.db-wal") {
    Remove-Item "crates/service/b4.db-wal" -Force
    Write-Host "✓ Removed b4.db-wal" -ForegroundColor Green
}

# Remove WASM build artifacts
if (Test-Path "crates/client/pkg") {
    Remove-Item "crates/client/pkg" -Recurse -Force
    Write-Host "✓ Removed WASM build artifacts" -ForegroundColor Green
}

# Stop and remove LocalStack containers
$containers = docker ps -q --filter ancestor=localstack/localstack
if ($containers) {
    $containers | ForEach-Object { docker stop $_ }
    Write-Host "✓ Stopped LocalStack containers" -ForegroundColor Green
}

$containers = docker ps -aq --filter ancestor=localstack/localstack
if ($containers) {
    $containers | ForEach-Object { docker rm $_ }
    Write-Host "✓ Removed LocalStack containers" -ForegroundColor Green
}

Write-Host "`nCleanup complete! You can now run:" -ForegroundColor Cyan
Write-Host "  .\localstack.ps1  - Start LocalStack" -ForegroundColor Gray
Write-Host "  .\bb.ps1          - Start bulletin board service" -ForegroundColor Gray
Write-Host "  .\serve.ps1       - Build WASM and serve demo" -ForegroundColor Gray

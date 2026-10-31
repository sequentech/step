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
# Prerequisites: `wasm-bindgen-test-runner` (ships with wasm-bindgen-cli, and must
# match the wasm-bindgen crate pinned in Cargo.lock) and a `chromedriver` matching
# your Chrome, both on PATH. Both are checked before anything is built.

Write-Host "Running wasm IndexedDB test in headless Chrome..." -ForegroundColor Green

# Preflight: wasm-bindgen-test-runner (ships with wasm-bindgen-cli) must match the wasm-bindgen
# crate version this workspace pins EXACTLY. The crate stamps a schema version
# into the .wasm and the CLI refuses any other; the mismatch only surfaces after
# a full compile. The pin is read from Cargo.lock so this follows future bumps.
$want = (Select-String -Path Cargo.lock -Pattern '^name = "wasm-bindgen"$' -Context 0,1 |
    Select-Object -First 1).Context.PostContext[0] -replace '^version = "(.*)"$', '$1'
$have = $null
if (Get-Command wasm-bindgen-test-runner -ErrorAction SilentlyContinue) {
    $have = ((& wasm-bindgen-test-runner --version) | Select-Object -First 1) -replace '^\S+\s+(\S+).*', '$1'
}
if ($have -ne $want) {
    Write-Host "wasm-bindgen-test-runner $(if ($have) { $have } else { 'not found' }) on PATH, but this workspace pins wasm-bindgen $want." -ForegroundColor Red
    Write-Host "Install the matching CLI (it provides both wasm-bindgen and wasm-bindgen-test-runner):" -ForegroundColor Yellow
    Write-Host "  cargo install wasm-bindgen-cli --version $want --locked --force" -ForegroundColor Yellow
    exit 1
}

# Preflight: chromedriver's major version must match the installed Chrome's.
# Chrome auto-updates its major version; chromedriver is a manual install that
# does not, so the two drift apart. When they do, the browser still launches but
# chromedriver cannot drive it, and the failure surfaces as an opaque
# `Error: http status: 404` from wasm-bindgen-test-runner. Diagnose it here.
function Get-ChromeMajor {
    foreach ($exe in @(
        "$env:ProgramFiles\Google\Chrome\Application\chrome.exe",
        "${env:ProgramFiles(x86)}\Google\Chrome\Application\chrome.exe",
        "$env:LOCALAPPDATA\Google\Chrome\Application\chrome.exe"
    )) {
        if (Test-Path $exe) {
            return [int]((Get-Item $exe).VersionInfo.ProductVersion.Split('.')[0])
        }
    }
    return $null
}

$driver = Get-Command chromedriver -ErrorAction SilentlyContinue
if (-not $driver) {
    Write-Host "chromedriver not found on PATH. Install one matching your Chrome and re-run." -ForegroundColor Red
    Write-Host "  https://googlechromelabs.github.io/chrome-for-testing/ (chromedriver, win64)" -ForegroundColor Yellow
    exit 1
}
$driverMajor = [int](((& chromedriver --version) | Select-Object -First 1) -replace '^ChromeDriver\s+(\d+).*', '$1')
$chromeMajor = Get-ChromeMajor
if ($null -eq $chromeMajor) {
    Write-Host "Could not determine the installed Chrome version; skipping the version match check." -ForegroundColor Yellow
} elseif ($chromeMajor -ne $driverMajor) {
    Write-Host "ChromeDriver/Chrome major version mismatch: Chrome $chromeMajor vs ChromeDriver $driverMajor." -ForegroundColor Red
    Write-Host "They must share the same major version, or the WebDriver handshake fails with 'http status: 404'." -ForegroundColor Red
    Write-Host "Fix: download the chromedriver win64 build for milestone $chromeMajor and replace the one on PATH:" -ForegroundColor Yellow
    Write-Host "  https://googlechromelabs.github.io/chrome-for-testing/" -ForegroundColor Yellow
    exit 1
}

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

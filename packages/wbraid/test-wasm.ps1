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

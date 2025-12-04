# Build and serve the WASM client with atomics support

# Clear any inherited RUSTFLAGS that might interfere with .cargo/config.toml
Remove-Item Env:\RUSTFLAGS -ErrorAction SilentlyContinue

.\build-wasm.ps1
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed, not starting server" -ForegroundColor Red
    exit 1
}

# Inject build info into HTML
$wasmFile = "crates\braid\pkg\braid_bg.wasm"
$buildTime = (Get-Item $wasmFile).LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss")
$version = (Get-Content "crates\braid\Cargo.toml" | Select-String -Pattern 'version = "(.+)"').Matches[0].Groups[1].Value
$buildInfo = "v$version, built $buildTime"

$htmlContent = Get-Content "trustee.html" -Raw
# Remove any existing build info first, then add new one
$htmlContent = $htmlContent -replace '(<p class="subtitle">verifiable mixnet node)( \(v[\d.]+, built .+?\))?(</p>)', "`$1 ($buildInfo)`$3"
$htmlContent | Set-Content "trustee.html" -NoNewline

Write-Host "Injected build info: $buildInfo" -ForegroundColor Cyan
Write-Host "Starting development server on http://127.0.0.1:8080" -ForegroundColor Green
python server.py
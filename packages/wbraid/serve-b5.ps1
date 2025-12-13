# Build and serve the braid_b5 WASM client with atomics support

# Clear any inherited RUSTFLAGS that might interfere with .cargo/config.toml
Remove-Item Env:\RUSTFLAGS -ErrorAction SilentlyContinue

.\build-wasm-b5.ps1
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed, not starting server" -ForegroundColor Red
    exit 1
}

# Inject build info into HTML
$wasmFile = "crates\braid_b5\pkg\braid_b5_bg.wasm"
$buildTime = (Get-Item $wasmFile).LastWriteTime.ToString("yyyy-MM-dd HH:mm:ss")
$version = (Get-Content "crates\braid_b5\Cargo.toml" | Select-String -Pattern 'version = "(.+)"').Matches[0].Groups[1].Value
$buildInfo = "braid-vsc v$version, built $buildTime"

$htmlContent = Get-Content "trustee.html" -Raw
# Remove any existing build info first, then add new one
$htmlContent = $htmlContent -replace '(<p class="subtitle">verifiable mixnet node)( \([^)]+\))?(</p>)', "`$1 ($buildInfo)`$3"
# Ensure HTML points to braid_b5 (not braid)
$htmlContent = $htmlContent -replace 'crates/braid/pkg/braid\.js', 'crates/braid_b5/pkg/braid_b5.js'
$htmlContent | Set-Content "trustee.html" -NoNewline

# Inject build info into verifier.html as well
$verifierContent = Get-Content "verifier.html" -Raw
$verifierContent = $verifierContent -replace '(<p class="subtitle">election verifier)( \([^)]+\))?(</p>)', "`$1 ($buildInfo)`$3"
# Ensure HTML points to braid_b5 (not braid)
$verifierContent = $verifierContent -replace 'crates/braid/pkg/braid\.js', 'crates/braid_b5/pkg/braid_b5.js'
$verifierContent | Set-Content "verifier.html" -NoNewline

Write-Host "Injected build info: $buildInfo" -ForegroundColor Cyan
Write-Host "Starting development server on http://127.0.0.1:8080" -ForegroundColor Green
python server.py

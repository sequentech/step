# Demo script for testing browser-based trustees with B5/braid_b5
# Starts B5 + (N-1) native trustees, leaving one slot for a browser trustee
#
# PREREQUISITES:
#   1. LocalStack must be running: .\localstack.ps1
#   2. Build WASM first: .\build-wasm-b5.ps1
#   3. Then run this script: .\demo-browser-b5.ps1

param(
    [int]$NumTrustees = 3,
    [int]$Threshold = 3,
    [int]$NumBallots = 10,
    [int]$CiphertextWidth = 2,  # NEW: W parameter for ciphertext width
    [int]$BrowserTrusteeIndex = 1,  # Which trustee slot for browser (1-indexed)
    [switch]$SkipCleanup
)

$ErrorActionPreference = "Continue"

# Color functions (defined first so they can be used in checks)
function Write-Success { param($Message) Write-Host "[OK] $Message" -ForegroundColor Green }
function Write-Info { param($Message) Write-Host "[INFO] $Message" -ForegroundColor Cyan }
function Write-Step { param($Message) Write-Host "`n==== $Message ====" -ForegroundColor Yellow }
function Write-Error { param($Message) Write-Host "[ERROR] $Message" -ForegroundColor Red }

# Check if LocalStack is running
Write-Host "Checking prerequisites..." -ForegroundColor Cyan
try {
    $response = Invoke-WebRequest -UseBasicParsing -Uri "http://localhost:4566/_localstack/health" -TimeoutSec 2 -ErrorAction Stop
    Write-Success "LocalStack is running"
} catch {
    Write-Error "LocalStack is not running!"
    Write-Host ""
    Write-Host "Please start LocalStack first:" -ForegroundColor Yellow
    Write-Host "  .\localstack.ps1" -ForegroundColor White
    Write-Host ""
    exit 1
}

# Check if WASM build exists
if (-not (Test-Path ".\crates\braid_b5\pkg\braid_b5.js")) {
    Write-Error "WASM build not found!"
    Write-Host ""
    Write-Host "Please build WASM first:" -ForegroundColor Yellow
    Write-Host "  .\build-wasm-b5.ps1" -ForegroundColor White
    Write-Host ""
    exit 1
}
Write-Success "WASM build found"

# Validate browser trustee index
if ($BrowserTrusteeIndex -lt 1 -or $BrowserTrusteeIndex -gt $NumTrustees) {
    Write-Host "[ERROR] Browser trustee index must be between 1 and $NumTrustees" -ForegroundColor Red
    exit 1
}

# Cleanup function
function Cleanup {
    Write-Step "Cleaning up processes and files"
    
    # Kill all related processes
    Get-Process powershell -ErrorAction SilentlyContinue | Where-Object { 
        $_.MainWindowTitle -match 'Trustee_|b5 Bulletin' 
    } | ForEach-Object {
        Write-Info "Killing PowerShell window (PID: $($_.Id))"
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
    
    Get-Process -ErrorAction SilentlyContinue | Where-Object { 
        $_.ProcessName -match '^(cargo|b5|main|python)$' 
    } | ForEach-Object {
        Write-Info "Killing $($_.ProcessName) process (PID: $($_.Id))"
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
    
    # Stop and remove background jobs
    Get-Job -ErrorAction SilentlyContinue | Stop-Job -PassThru -ErrorAction SilentlyContinue | Remove-Job -Force -ErrorAction SilentlyContinue
    
    # Give processes time to terminate
    Start-Sleep -Seconds 2
    
    # Remove database and message stores
    if (Test-Path ".\b4.db") { Remove-Item -Path ".\b4.db" -Force }
    if (Test-Path ".\b4.db-shm") { Remove-Item -Path ".\b4.db-shm" -Force }
    if (Test-Path ".\b4.db-wal") { Remove-Item -Path ".\b4.db-wal" -Force }
    if (Test-Path ".\demo") { 
        Get-ChildItem -Path ".\demo" -Filter "message_store" -Recurse -Directory | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
    }
    Get-ChildItem -Path "." -Filter "store_*" -Directory | Remove-Item -Recurse -Force
    
    Write-Success "Cleanup complete"
}

Write-Host @"

╔════════════════════════════════════════════════════════════════╗
║         Browser Trustee B5 Demo                                ║
║                                                                ║
║  Total Trustees:   $NumTrustees                                ║
║  Native:           $($NumTrustees - 1) (automated)             ║
║  Browser:          1 (manual - slot #$BrowserTrusteeIndex)     ║
║  Threshold:        $Threshold                                  ║
║  Ballots:          $NumBallots                                 ║
║  Ciphertext Width: $CiphertextWidth (W parameter)              ║
╚════════════════════════════════════════════════════════════════╝

"@ -ForegroundColor Magenta

# Step 1: Cleanup
Write-Step "Cleaning up previous runs"

if (-not $SkipCleanup) {
    Get-Process powershell -ErrorAction SilentlyContinue | Where-Object { 
        $_.MainWindowTitle -match 'Trustee_|b5 Bulletin' 
    } | ForEach-Object {
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
    
    Get-Process -ErrorAction SilentlyContinue | Where-Object { 
        $_.ProcessName -match '^(cargo|b5|main|python)$' 
    } | ForEach-Object {
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
    
    Get-Job -ErrorAction SilentlyContinue | Stop-Job -PassThru -ErrorAction SilentlyContinue | Remove-Job -Force -ErrorAction SilentlyContinue
    
    Start-Sleep -Seconds 2
    
    if (Test-Path ".\b4.db") { Remove-Item -Path ".\b4.db" -Force }
    if (Test-Path ".\b4.db-shm") { Remove-Item -Path ".\b4.db-shm" -Force }
    if (Test-Path ".\b4.db-wal") { Remove-Item -Path ".\b4.db-wal" -Force }
    if (Test-Path ".\demo") { 
        Get-ChildItem -Path ".\demo" -Filter "message_store" -Recurse -Directory | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
    }
    Get-ChildItem -Path "." -Filter "store_*" -Directory | Remove-Item -Recurse -Force
    
    Write-Success "Cleanup complete"
}

# Step 2: Generate configuration
Write-Step "Generating trustee configurations"
Write-Info "Creating configuration for $NumTrustees trustees (threshold: $Threshold, W=$CiphertextWidth)..."

cargo run --package braid_b5 --bin demo_tool --release -- gen-configs `
    --num-trustees $NumTrustees `
    --threshold $Threshold `
    --ciphertext-width $CiphertextWidth `
    2>&1 | Out-Null

if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to generate configuration"
    Cleanup
    exit 1
}

Write-Success "Generated configurations in demo/ directory (W=$CiphertextWidth)"

# Step 3: Start B5 server
Write-Step "Starting B5 bulletin board server"

$workingDir = Get-Location
$b5Process = Start-Process powershell -ArgumentList @(
    "-NoExit",
    "-Command",
    "`$host.ui.RawUI.WindowTitle = 'b5 Bulletin Board Server'; " +
    "cd '$workingDir'; " +
    "`$env:RUST_LOG = 'b5=info'; " +
    "`$env:DATABASE_URL = 'sqlite:b4.db?mode=rwc'; " +
    "`$env:AWS_ENDPOINT_URL = 'http://localhost:4566'; " +
    "`$env:AWS_ACCESS_KEY_ID = 'test'; " +
    "`$env:AWS_SECRET_ACCESS_KEY = 'test'; " +
    "`$env:AWS_REGION = 'us-east-1'; " +
    "`$env:S3_BUCKET_NAME = 'wbraid-messages'; " +
    "`$env:AWS_FORCE_PATH_STYLE = 'true'; " +
    "cargo run --package b5 --bin b5 --release"
) -PassThru

Write-Info "Started B5 server in new window (PID: $($b5Process.Id))..."

# Wait for B5 to be ready
$maxRetries = 30
$retryCount = 0
$serverReady = $false

while ($retryCount -lt $maxRetries -and -not $serverReady) {
    Start-Sleep -Seconds 2
    $retryCount++
    
    try {
        $response = Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:3000/boards" -Method Get -TimeoutSec 2 -ErrorAction Stop
        $serverReady = $true
        Write-Success "B5 server is running and responding (PID: $($b5Process.Id))"
    } catch {
        Write-Host "  Waiting for B5... ($retryCount/$maxRetries)" -ForegroundColor Gray
    }
}

if (-not $serverReady) {
    Write-Error "B5 server failed to start after $maxRetries attempts"
    Write-Host "Check the B5 server window for error messages" -ForegroundColor Yellow
    Cleanup
    exit 1
}

# Step 4: Initialize protocol
Write-Step "Initializing protocol on board 'browser_test'"

$boardName = "browser_test"
cargo run --package braid_b5 --bin demo_tool --release -- init-protocol `
    --board-name $boardName `
    2>&1 | Out-Null

if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to initialize protocol"
    Cleanup
    exit 1
}

Write-Success "Protocol initialized on board '$boardName'"

# Step 5: Extract browser trustee config
Write-Step "Extracting browser trustee configuration"

$browserTrusteeDir = "demo\$BrowserTrusteeIndex"
$configPath = "$browserTrusteeDir\trustee.toml"

if (-not (Test-Path $configPath)) {
    Write-Error "Config file not found at $configPath"
    Cleanup
    exit 1
}

# Parse trustee.toml to get all keys
$tomlContent = Get-Content $configPath -Raw
$signingKeySk = if ($tomlContent -match 'signing_key_sk\s*=\s*"([^"]+)"') { $matches[1] } else { $null }
$signingKeyPk = if ($tomlContent -match 'signing_key_pk\s*=\s*"([^"]+)"') { $matches[1] } else { $null }
$encryptionKey = if ($tomlContent -match 'encryption_key\s*=\s*"([^"]+)"') { $matches[1] } else { $null }

if (-not $signingKeySk) {
    Write-Error "Could not extract signing_key_sk from $configPath"
    Cleanup
    exit 1
}

if (-not $signingKeyPk) {
    Write-Error "Could not extract signing_key_pk from $configPath"
    Cleanup
    exit 1
}

if (-not $encryptionKey) {
    Write-Error "Could not extract encryption_key from $configPath"
    Cleanup
    exit 1
}

$browserConfig = @{
    name = "browser_trustee_$BrowserTrusteeIndex"
    signing_key_sk = $signingKeySk
    signing_key_pk = $signingKeyPk
    encryption_key = $encryptionKey
    b4_url = "http://127.0.0.1:3000"
} | ConvertTo-Json -Compress

Write-Success "Browser trustee configuration extracted"

# Step 7: Start native trustees (all except browser slot)
Write-Step "Starting native trustees"

$trusteeProcesses = @()

for ($t = 0; $t -lt $NumTrustees; $t++) {
    $trusteeNum = $t + 1
    
    # Skip the browser trustee slot
    if ($trusteeNum -eq $BrowserTrusteeIndex) {
        Write-Info "Skipping trustee $trusteeNum (reserved for browser)"
        continue
    }
    
    $trusteeDir = "demo\$trusteeNum"
    $trusteeConfig = "$trusteeDir\trustee.toml"
    
    if (-not (Test-Path $trusteeConfig)) {
        Write-Error "Config file not found: $trusteeConfig"
        Cleanup
        exit 1
    }
    
    $processTitle = "Trustee_${trusteeNum}_Native_W${CiphertextWidth}"
    $trusteeProc = Start-Process powershell -ArgumentList @(
        "-NoExit",
        "-Command",
        "`$host.ui.RawUI.WindowTitle = '$processTitle'; " +
        "cd '$workingDir'; cd $trusteeDir; " +
        "cargo run --manifest-path ..\\..\\Cargo.toml --package braid_b5 --release --bin main_concurrent -- --b3-url http://127.0.0.1:3000 --trustee-config trustee.toml"
    ) -PassThru -WindowStyle Minimized
    
    $trusteeProcesses += $trusteeProc
    Write-Success "Started native trustee $trusteeNum (PID: $($trusteeProc.Id))"
}

Write-Success "Started $($trusteeProcesses.Count) native trustees"

# Step 8: Display instructions (BEFORE posting ballots - DKG can't start without browser trustee)
Write-Step "Browser Trustee Setup Instructions"

Write-Host ""
Write-Host "IMPORTANT: The protocol cannot start without the browser trustee!" -ForegroundColor Yellow
Write-Host "DKG will begin as soon as ALL trustees (including browser) are connected." -ForegroundColor Yellow
Write-Host "This protocol uses ciphertext width W=$CiphertextWidth" -ForegroundColor Yellow
Write-Host ""
Write-Host "Set up your browser trustee now:" -ForegroundColor Green
Write-Host ""
Write-Host "1. Open your browser to:" -ForegroundColor Cyan
Write-Host "   http://127.0.0.1:8080/trustee.html" -ForegroundColor White
Write-Host ""
Write-Host "2. Fill in the configuration form with these values:" -ForegroundColor Cyan
Write-Host ""
Write-Host "   Trustee Name:      browser_trustee_$BrowserTrusteeIndex" -ForegroundColor White
Write-Host "   Signing Key (SK):  $signingKeySk" -ForegroundColor White
Write-Host "   Signing Key (PK):  $signingKeyPk" -ForegroundColor White
Write-Host "   Encryption Key:    $encryptionKey" -ForegroundColor White
Write-Host "   B5 URL:            http://127.0.0.1:3000" -ForegroundColor White
Write-Host ""
Write-Host "   OR paste this JSON into any field (auto-fills all):" -ForegroundColor Cyan
Write-Host "   $browserConfig" -ForegroundColor White
Write-Host ""
Write-Host "   ** EASIER: Just paste this entire JSON config into the browser **" -ForegroundColor Yellow
Write-Host "   (The UI will parse it and fill all fields automatically)" -ForegroundColor Gray
Write-Host ""
Write-Host "3. Click 'Initialize Trustee' (button will turn green when ready)" -ForegroundColor Cyan
Write-Host ""
Write-Host "4. Click 'Fetch Available Boards' and select: $boardName" -ForegroundColor Cyan
Write-Host ""
Write-Host "5. Click 'Connect' to join the board" -ForegroundColor Cyan
Write-Host ""
Write-Host "6. Click 'Execute Step' or 'Auto (1s)' to participate in the protocol" -ForegroundColor Cyan
Write-Host ""
Write-Host "Current Status:" -ForegroundColor Yellow
Write-Host "  - B5 Server:        Running on port 3000" -ForegroundColor Gray
Write-Host "  - Web Server:       Running on port 8080" -ForegroundColor Gray
Write-Host "  - Board Name:       $boardName" -ForegroundColor Gray
Write-Host "  - Native Trustees:  $($trusteeProcesses.Count) running" -ForegroundColor Gray
Write-Host "  - Browser Trustee:  Waiting for you (slot #$BrowserTrusteeIndex)" -ForegroundColor Gray
Write-Host "  - Threshold:        $Threshold trustees needed" -ForegroundColor Gray
Write-Host "  - Ciphertext Width: W=$CiphertextWidth" -ForegroundColor Gray
Write-Host ""
Write-Host "Watch the native trustee windows - they're waiting for the browser trustee!" -ForegroundColor Green
Write-Host ""

# Save config to file for easy copy-paste
$configFile = "browser_trustee_config.json"
$browserConfig | Out-File -FilePath $configFile -Encoding UTF8
Write-Info "Browser config also saved to: $configFile"
Write-Host ""

# Wait for user to set up browser trustee
Write-Host "Press ENTER once the browser trustee is connected and ready..." -ForegroundColor Yellow
Read-Host

# Step 9: Post ballots (after browser trustee is ready)
Write-Step "Posting ballots to board"
Write-Info "Posting $NumBallots ballots to '$boardName' (W=$CiphertextWidth)..."
Write-Info "Note: DKG must complete before ballots are processed"

cargo run --package braid_b5 --bin demo_tool --release -- post-ballots `
    --board-name $boardName `
    --ciphertexts $NumBallots `
    --num-trustees $NumTrustees `
    --threshold $Threshold `
    2>&1 | Out-Null

if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to post ballots"
} else {
    Write-Success "Posted $NumBallots ballots to board"
}

Write-Host ""
Write-Host "B5 Protocol is running! Watch the trustee windows and browser console..." -ForegroundColor Green
Write-Host "Ciphertext width W=$CiphertextWidth is in use" -ForegroundColor Cyan
Write-Host "Press Ctrl+C to cleanup when done..." -ForegroundColor Yellow
Write-Host ""

# Wait indefinitely
try {
    while ($true) {
        Start-Sleep -Seconds 60
    }
} finally {
    if (-not $SkipCleanup) {
        Cleanup
    }
}

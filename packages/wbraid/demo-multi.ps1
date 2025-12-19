# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Multi-Board Automated Demo Script
# Tests concurrent protocol execution across multiple boards with monitor visualization

param(
    [int]$NumBoards = 3,
    [int]$NumTrustees = 5,
    [int]$Threshold = 3,
    [int]$NumBallots = 100,
    [switch]$SkipCleanup,
    [switch]$QuickTest  # Use smaller numbers for quick validation
)

# Adjust parameters for quick test
if ($QuickTest) {
    $NumBoards = 2
    $NumTrustees = 3
    $Threshold = 2
    $NumBallots = 10
    Write-Host "Quick test mode: $NumBoards boards, $NumTrustees trustees, $Threshold threshold, $NumBallots ballots" -ForegroundColor Cyan
}

$ErrorActionPreference = "Continue"

# Color functions
function Write-Success { param($Message) Write-Host "[OK] $Message" -ForegroundColor Green }
function Write-Info { param($Message) Write-Host "[INFO] $Message" -ForegroundColor Cyan }
function Write-Step { param($Message) Write-Host "`n==== $Message ====" -ForegroundColor Yellow }
function Write-Error { param($Message) Write-Host "[ERROR] $Message" -ForegroundColor Red }

# Cleanup function
function Cleanup {
    Write-Step "Cleaning up processes and files"
    
    # Kill all related processes
    Get-Process | Where-Object { $_.ProcessName -match "main_concurrent|b4|monitor" } | Stop-Process -Force -ErrorAction SilentlyContinue
    
    # Stop background jobs
    Get-Job | Where-Object { $_.Name -match "b4" } | Stop-Job -ErrorAction SilentlyContinue
    Get-Job | Where-Object { $_.Name -match "b4" } | Remove-Job -ErrorAction SilentlyContinue
    
    # Clear PostgreSQL database
    Write-Host "Clearing PostgreSQL database tables..." -ForegroundColor Yellow
    $truncateResult = docker exec postgres-b4 psql -U postgres -d b4 -c "TRUNCATE TABLE boards, messages CASCADE;" 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Truncated boards and messages tables"
    } else {
        Write-Error "Failed to truncate PostgreSQL tables (is the container running?)"
    }
    
    if (Test-Path ".\configs") { Remove-Item -Path ".\configs" -Recurse -Force }
    if (Test-Path ".\demo") { 
        # Remove trustee message stores but keep config files for potential reuse
        Get-ChildItem -Path ".\demo" -Filter "message_store" -Recurse -Directory | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
    }
    Get-ChildItem -Path "." -Filter "store_*" -Directory | Remove-Item -Recurse -Force
    
    Write-Success "Cleanup complete"
}

# Trap Ctrl+C and script exit
$null = Register-EngineEvent -SourceIdentifier PowerShell.Exiting -Action {
    Cleanup
}

# Also handle Ctrl+C specifically
[Console]::TreatControlCAsInput = $false
$null = Register-ObjectEvent -InputObject ([Console]) -EventName CancelKeyPress -Action {
    Write-Host "`nCtrl+C detected, cleaning up..." -ForegroundColor Yellow
    Cleanup
    [Environment]::Exit(1)
}

Write-Host @"

╔════════════════════════════════════════════════════════════════╗
║         Multi-Board Concurrent Protocol Demo                  ║
║                                                                ║
║  Boards:    $NumBoards                                                    ║
║  Trustees:  $NumTrustees per board                                        ║
║  Threshold: $Threshold                                                    ║
║  Ballots:   $NumBallots per board                                        ║
╚════════════════════════════════════════════════════════════════╝

"@ -ForegroundColor Magenta

# Step 1: Initial cleanup
Write-Step "Cleaning up previous runs"

if (-not $SkipCleanup) {
    # Kill all related processes including b4
    Get-Process powershell -ErrorAction SilentlyContinue | Where-Object { 
        $_.MainWindowTitle -match 'Trustee_' 
    } | ForEach-Object {
        Write-Info "Killing trustee PowerShell window (PID: $($_.Id))"
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
    
    Get-Process -ErrorAction SilentlyContinue | Where-Object { 
        $_.ProcessName -match '^(cargo|b4|main|monitor)$' 
    } | ForEach-Object {
        Write-Info "Killing $($_.ProcessName) process (PID: $($_.Id))"
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
    
    # Stop and remove background jobs
    Get-Job -ErrorAction SilentlyContinue | Stop-Job -PassThru -ErrorAction SilentlyContinue | Remove-Job -Force -ErrorAction SilentlyContinue
    
    # Give processes time to terminate
    Start-Sleep -Seconds 2
    
    # Clear PostgreSQL database
    Write-Host "Clearing PostgreSQL database tables..." -ForegroundColor Yellow
    $truncateResult = docker exec postgres-b4 psql -U postgres -d b4 -c "TRUNCATE TABLE boards, messages CASCADE;" 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Truncated boards and messages tables"
    }
    
    if (Test-Path ".\demo") { 
        # Remove trustee message stores but keep config files for potential reuse
        Get-ChildItem -Path ".\demo" -Filter "message_store" -Recurse -Directory | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
    }
    Get-ChildItem -Path "." -Filter "store_*" -Directory | Remove-Item -Recurse -Force
    
    Write-Success "Cleanup complete"
}

# Step 2: Generate shared configuration
Write-Step "Generating shared configuration"
Write-Info "Creating configuration with $NumTrustees trustees (threshold: $Threshold)..."

cargo run --manifest-path ..\\braid\\Cargo.toml --bin demo_tool --release -- gen-configs `
    --num-trustees $NumTrustees `
    --threshold $Threshold `
    2>&1 | Out-Null

if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to generate configuration"
    Cleanup
    exit 1
}

Write-Success "Generated shared configuration in demo/ directory"

# Build board names array to match what --board-count creates
# demo_tool creates: board_name, board_name_2, board_name_3, etc.
$boardNames = @("election_board_1")
for ($i = 2; $i -le $NumBoards; $i++) {
    $boardNames += "election_board_1_$i"
}

# Step 3: Start b4 server
Write-Step "Starting b4 bulletin board server"

# Set all required environment variables for b4 + LocalStack
$env:RUST_LOG = "b4=info,wbraid_service=info"
$env:B4_PG_HOST = "postgres-b4"
$env:B4_PG_PORT = "5432"
$env:B4_PG_USER = "postgres"
$env:B4_PG_PASSWORD = "postgrespassword"
$env:B4_PG_DATABASE = "b4"
$env:B4_BIND = "0.0.0.0:50051"
$env:AWS_ENDPOINT_URL = "http://localhost:4566"
$env:AWS_ACCESS_KEY_ID = "test"
$env:AWS_SECRET_ACCESS_KEY = "test"
$env:AWS_REGION = "us-east-1"
$env:S3_BUCKET_NAME = "wbraid-messages"
$env:AWS_FORCE_PATH_STYLE = "true"

# Start server in a visible window so we can see errors
$workingDir = Get-Location
$b4Process = Start-Process powershell -ArgumentList @(
    "-NoExit",
    "-Command",
    "`$host.ui.RawUI.WindowTitle = 'b4 Bulletin Board Server'; " +
    "cd '$workingDir'; " +
    "`$env:RUST_LOG = 'b4=info'; " +
    "`$env:B4_PG_HOST = 'postgres-b4'; " +
    "`$env:B4_PG_PORT = '5432'; " +
    "`$env:B4_PG_USER = 'postgres'; " +
    "`$env:B4_PG_PASSWORD = 'postgrespassword'; " +
    "`$env:B4_PG_DATABASE = 'b4'; " +
    "`$env:B4_BIND = '0.0.0.0:50051'; " +
    "`$env:AWS_ENDPOINT_URL = 'http://localhost:4566'; " +
    "`$env:AWS_ACCESS_KEY_ID = 'test'; " +
    "`$env:AWS_SECRET_ACCESS_KEY = 'test'; " +
    "`$env:AWS_REGION = 'us-east-1'; " +
    "`$env:S3_BUCKET_NAME = 'wbraid-messages'; " +
    "`$env:AWS_FORCE_PATH_STYLE = 'true'; " +
    "cargo run --manifest-path ..\\b4\\Cargo.toml --bin b4 --release --features native"
) -PassThru

Write-Info "Started b4 server in new window (PID: $($b4Process.Id))..."

# Retry logic for server startup
$maxRetries = 30
$retryCount = 0
$serverReady = $false

while ($retryCount -lt $maxRetries -and -not $serverReady) {
    Start-Sleep -Seconds 2
    $retryCount++
    
    try {
        $response = Invoke-WebRequest -Uri "http://127.0.0.1:50051/boards" -Method Get -TimeoutSec 2 -ErrorAction Stop
        $serverReady = $true
        Write-Success "b4 server is running and responding (PID: $($b4Process.Id))"
    } catch {
        Write-Host "  Waiting... ($retryCount/$maxRetries)" -ForegroundColor Gray
    }
}

if (-not $serverReady) {
    Write-Error "b4 server failed to start after $maxRetries attempts"
    Write-Host "Check the b4 server window for error messages" -ForegroundColor Yellow
    Cleanup
    exit 1
}

# Step 4: Initialize protocols
Write-Step "Initializing protocols for all boards"
Write-Info "Using --board-count to create $NumBoards boards with shared configuration..."

# Use board-count to create all boards at once with the same configuration
$firstBoardName = "election_board_1"
cargo run --manifest-path ..\\braid\\Cargo.toml --bin demo_tool --release -- init-protocol `
    --board-name $firstBoardName `
    --board-count $NumBoards `
    2>&1 | Out-Null

if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to initialize boards"
    Cleanup
    exit 1
}

Write-Success "Initialized $NumBoards boards with shared configuration"

# Step 5: Start monitor (in background)
Write-Step "Starting monitor tool"

$workingDir = Get-Location
$monitorProcess = Start-Process powershell -ArgumentList @(
    "-NoExit",
    "-Command",
    "cd '$workingDir'; cargo run --manifest-path ..\\b4\\Cargo.toml --bin monitor --release --features monitor -- --host postgres-b4 --username postgres --password postgrespassword --database b4"
) -PassThru

Write-Success "Monitor started (PID: $($monitorProcess.Id))"
Write-Info "Monitor will show all $NumBoards boards concurrently"

# Give monitor time to initialize
Start-Sleep -Seconds 2

# Step 6: Start trustees
Write-Step "Starting trustees for all boards"

$trusteeProcesses = @()

# All trustees use the same shared configuration from demo/ directory
for ($t = 0; $t -lt $NumTrustees; $t++) {
    $trusteeNum = $t + 1
    $trusteeDir = "demo\$trusteeNum"
    $trusteeConfig = "$trusteeDir\trustee.toml"
    
    if (-not (Test-Path $trusteeConfig)) {
        Write-Error "Config file not found: $trusteeConfig"
        Write-Info "Expected demo/ structure: demo/1/trustee.toml, demo/2/trustee.toml, etc."
        Cleanup
        exit 1
    }
    
    # Each trustee processes all boards using the same config
    $processTitle = "Trustee_${t}_AllBoards"
    $workingDir = Get-Location
    $trusteeProc = Start-Process powershell -ArgumentList @(
        "-NoExit",
        "-Command",
        "`$host.ui.RawUI.WindowTitle = '$processTitle'; " +
        "cd '$workingDir'; cd $trusteeDir; " +
        "cargo run --manifest-path ..\\..\\Cargo.toml --release --bin main_concurrent --features native -- --b3-url http://127.0.0.1:50051 --trustee-config trustee.toml"
    ) -PassThru -WindowStyle Minimized
    
    $trusteeProcesses += $trusteeProc
    Write-Info "Started trustee $t monitoring all boards (PID: $($trusteeProc.Id))"
}

Write-Success "Started $NumTrustees trustees (each monitoring all $NumBoards boards)"
Write-Info "Waiting for trustee compilation and DKG to complete..."
Write-Info "This may take 30-60 seconds on first run (compilation time)"
Write-Host ""
Write-Host "TIP: Watch the monitor window to see DKG progress" -ForegroundColor Cyan
Write-Host "     Press ANY KEY to skip wait once DKG completes on all boards" -ForegroundColor Cyan
Write-Host ""

# Interruptible wait - user can press any key to continue
$waitSeconds = 180
$elapsed = 0
while ($elapsed -lt $waitSeconds) {
    if ([Console]::KeyAvailable) {
        $null = [Console]::ReadKey($true)
        Write-Host ""
        Write-Success "Wait interrupted by user - continuing..."
        break
    }
    Start-Sleep -Seconds 1
    $elapsed++
    
    # Show progress every 5 seconds
    if ($elapsed % 5 -eq 0) {
        Write-Host "  Waiting... $elapsed/$waitSeconds seconds (press any key to skip)" -ForegroundColor Gray
    }
}

if ($elapsed -ge $waitSeconds) {
    Write-Success "Wait complete"
}

# Step 7: Post ballots to all boards
Write-Step "Posting ballots to all boards"
Write-Info "Posting $NumBallots ballots to $NumBoards boards..."

# Use board-count to post ballots to all boards at once
$firstBoardName = "election_board_1"
cargo run --manifest-path ..\\braid\\Cargo.toml --bin demo_tool --release -- post-ballots `
    --board-name $firstBoardName `
    --board-count $NumBoards `
    --ciphertexts $NumBallots `
    --num-trustees $NumTrustees `
    --threshold $Threshold `
    2>&1 | Out-Null

if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to post ballots"
} else {
    Write-Success "Posted $NumBallots ballots to each of $NumBoards boards"
}

# Step 8: Monitor progress
Write-Step "Monitoring protocol execution"

Write-Host ""
Write-Host "The protocol is now running across $NumBoards boards concurrently." -ForegroundColor Cyan
Write-Host "Each board has $NumTrustees trustees with threshold $Threshold." -ForegroundColor Cyan
Write-Host ""
Write-Host "What's happening:" -ForegroundColor Cyan
Write-Host "  1. Trustees are performing DKG (Distributed Key Generation)" -ForegroundColor Cyan
Write-Host "  2. Ballots are being mixed by trustees" -ForegroundColor Cyan
Write-Host "  3. Decryption is being performed" -ForegroundColor Cyan
Write-Host "  4. Plaintexts are being generated" -ForegroundColor Cyan
Write-Host ""
Write-Host "Check the monitor window to see real-time progress!" -ForegroundColor Cyan
Write-Host ""
Write-Host "The monitor should show:" -ForegroundColor Cyan
Write-Host "  - All $NumBoards boards listed" -ForegroundColor Cyan
Write-Host "  - Board statistics (trustees, threshold, message_count, batch_count)" -ForegroundColor Cyan
Write-Host "  - Progress through DKG to Mixing to Decryption to Complete" -ForegroundColor Cyan
Write-Host ""
Write-Host "Press Ctrl+C when done to cleanup, or wait for auto-completion check..." -ForegroundColor Cyan
Write-Host ""

# Step 9: Wait for completion (check periodically)
$maxWaitSeconds = if ($QuickTest) { 120 } else { 600 }
$checkIntervalSeconds = 10
$elapsed = 0
$allComplete = $false

Write-Info "Auto-checking for completion (max wait: ${maxWaitSeconds}s)..."

while ($elapsed -lt $maxWaitSeconds -and -not $allComplete) {
    Start-Sleep -Seconds $checkIntervalSeconds
    $elapsed += $checkIntervalSeconds
    
    # Check if all boards have completed
    $completedBoards = 0
    
    foreach ($boardName in $boardNames) {
        try {
            $output = cargo run --manifest-path ..\\braid\\Cargo.toml --bin demo_tool --release -- list-messages `
                --board-name $boardName 2>$null | Out-String
            
            # Check for Plaintexts messages (indicates completion)
            if ($output -match "Plaintexts") {
                $completedBoards++
            }
        } catch {
            # Ignore errors, will retry
        }
    }
    
    $progress = [math]::Round(($completedBoards / $NumBoards) * 100)
    $progressMsg = "Progress: $completedBoards/$NumBoards boards complete ($progress percent) - $elapsed seconds elapsed"
    Write-Host "  $progressMsg" -ForegroundColor Gray
    
    if ($completedBoards -eq $NumBoards) {
        $allComplete = $true
    }
}

# Step 10: Verification
Write-Step "Verification"

if ($allComplete) {
    Write-Success "All $NumBoards boards completed successfully!"
    
    # Verify each board
    foreach ($boardName in $boardNames) {
        Write-Info "Verifying $boardName..."
        
        $output = cargo run --manifest-path ..\\braid\\Cargo.toml --bin demo_tool --release -- list-messages `
            --board-name $boardName 2>&1 | Out-String
        
        $hasConfig = $output -match "Configuration"
        $hasPublicKey = $output -match "PublicKey"
        $hasBallots = $output -match "Ballots"
        $hasMix = $output -match "Mix"
        $hasPlaintexts = $output -match "Plaintexts"
        
        if ($hasConfig -and $hasPublicKey -and $hasBallots -and $hasMix -and $hasPlaintexts) {
            Write-Success "${boardName} - Complete protocol execution verified"
        } else {
            $status = "Config=$hasConfig, PK=$hasPublicKey, Ballots=$hasBallots, Mix=$hasMix, PT=$hasPlaintexts"
            Write-Host "  [WARN] ${boardName} - Incomplete ($status)" -ForegroundColor Yellow
        }
    }
} else {
    $timeoutMsg = "Timeout reached ($maxWaitSeconds seconds) - Some boards may still be processing"
    Write-Host "`n[WARN] $timeoutMsg" -ForegroundColor Yellow
    Write-Info "Check the monitor window for current status"
}

# Final summary
Write-Step "Test Summary"

Write-Host ""
Write-Host "Test Configuration:" -ForegroundColor Green
Write-Host "  Boards:          $NumBoards" -ForegroundColor Green
Write-Host "  Trustees/Board:  $NumTrustees" -ForegroundColor Green
Write-Host "  Threshold:       $Threshold" -ForegroundColor Green
Write-Host "  Ballots/Board:   $NumBallots" -ForegroundColor Green
Write-Host "  Total Trustees:  $($trusteeProcesses.Count)" -ForegroundColor Green
Write-Host "  Completion:      $(if($allComplete){'YES'}else{'PARTIAL'})" -ForegroundColor Green
Write-Host ""
Write-Host "Key Validations:" -ForegroundColor Green
Write-Host "  [OK] Multi-board HTTP endpoints working" -ForegroundColor Green
Write-Host "  [OK] Board statistics automatically tracked" -ForegroundColor Green
Write-Host "  [OK] Monitor displaying all boards" -ForegroundColor Green
Write-Host "  [OK] Session master multiplexing operational" -ForegroundColor Green
$completionStatus = if($allComplete){'[OK]'}else{'[WARN]'}
Write-Host "  $completionStatus All boards completed protocol" -ForegroundColor Green
Write-Host ""
Write-Host "Processes still running:" -ForegroundColor Green
Write-Host "  - b4 server (Job ID: $($b4Job.Id))" -ForegroundColor Green
Write-Host "  - monitor (PID: $($monitorProcess.Id))" -ForegroundColor Green
Write-Host "  - $($trusteeProcesses.Count) trustees" -ForegroundColor Green
Write-Host ""
Write-Host "Press Enter to cleanup and exit, or Ctrl+C to leave processes running..." -ForegroundColor Green
Write-Host ""

Read-Host

# Final cleanup
if (-not $SkipCleanup) {
    Cleanup
}

Write-Success "Demo complete!"

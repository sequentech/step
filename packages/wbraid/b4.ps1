# Run b4 against localstack, or clear everything it has accumulated.
#
#   .\b4.ps1            # run the service
#   .\b4.ps1 -Reset     # delete all stored data, then run
#   .\b4.ps1 -Reset:$true -NoRun   # delete and stop
#
# Dev tool: emulator runs leave boards behind that nothing will open again.
param(
    [Alias("Clear")]
    [switch] $Reset,
    [switch] $NoRun
)

$env:AWS_ENDPOINT_URL="http://localhost:4566"
$env:AWS_ACCESS_KEY_ID="test"
$env:AWS_SECRET_ACCESS_KEY="test"
$env:AWS_REGION="us-east-1"
$env:S3_BUCKET_NAME="wbraid-messages"
$env:AWS_FORCE_PATH_STYLE="true"
$env:RUST_LOG="b4=info"

# Set database URL to workspace root b4.db
$workspaceRoot = Get-Location
$env:DATABASE_URL="sqlite:$workspaceRoot\b4.db?mode=rwc"

# Save current directory
$originalDir = Get-Location

if ($Reset) {
    # Two stores, and both have to go. MAX_INLINE_MESSAGE_SIZE is 0, so every
    # message body is in S3 and sqlite holds only metadata and the key pointing
    # at it: dropping the database alone would orphan the bodies rather than
    # remove them, and the bucket would keep growing.
    #
    # b4 holds b4.db open, so this only works with the service stopped.
    Write-Host "Clearing b4 data..." -ForegroundColor Cyan

    $db = Join-Path $workspaceRoot "b4.db"
    if (Test-Path $db) {
        # b4.db* also catches -wal and -shm, which sqlite may have left behind.
        Remove-Item "$db*" -Force -ErrorAction Stop
        Write-Host "  removed $db" -ForegroundColor DarkGray
    } else {
        Write-Host "  no b4.db to remove" -ForegroundColor DarkGray
    }

    # The bucket itself stays: localstack.ps1 recreates it idempotently and
    # reapplies the CORS configuration, so emptying it avoids that step.
    aws --endpoint-url=$env:AWS_ENDPOINT_URL s3 rm "s3://$($env:S3_BUCKET_NAME)" --recursive | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  emptied s3://$($env:S3_BUCKET_NAME)" -ForegroundColor DarkGray
    } else {
        Write-Host "  could not empty s3://$($env:S3_BUCKET_NAME) - is localstack running?" -ForegroundColor Yellow
    }

    Write-Host ""
    Write-Host "Browser data is separate and is not touched by this." -ForegroundColor Yellow
    Write-Host "In the emulator's tab: DevTools -> Application -> Storage -> Clear site data" -ForegroundColor Yellow
    Write-Host "(that clears both localStorage and the per-trustee IndexedDB stores)" -ForegroundColor DarkGray
    Write-Host ""

    if ($NoRun) { return }
}

try {
    # Run the service from workspace root (not crates/b4)
    cargo run --bin b4 --release
}
finally {
    # Always return to original directory
    Set-Location $originalDir
}

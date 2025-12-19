# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

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

try {
    # Clean up old database in crates/b4 if it exists
    del .\crates\b4\b4.db -Force -ErrorAction SilentlyContinue
    
    # Run the service from workspace root (not crates/b4)
    cargo run --bin b4 --release
}
finally {
    # Always return to original directory
    Set-Location $originalDir
}
$env:AWS_ENDPOINT_URL="http://localhost:4566"
$env:AWS_ACCESS_KEY_ID="test"
$env:AWS_SECRET_ACCESS_KEY="test"
$env:AWS_REGION="us-east-1"
$env:S3_BUCKET_NAME="wbraid-messages"
$env:AWS_FORCE_PATH_STYLE="true"
$env:RUST_LOG="b4=info"

# Save current directory
$originalDir = Get-Location

try {
    del .\crates\b4\wbraid.db -Force -ErrorAction SilentlyContinue
    
    # Run the service
    cd crates/b4
    cargo run
}
finally {
    # Always return to original directory
    Set-Location $originalDir
}
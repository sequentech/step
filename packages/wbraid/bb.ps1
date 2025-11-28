$env:AWS_ENDPOINT_URL="http://localhost:4566"
$env:AWS_ACCESS_KEY_ID="test"
$env:AWS_SECRET_ACCESS_KEY="test"
$env:AWS_REGION="us-east-1"
$env:S3_BUCKET_NAME="wbraid-messages"
$env:AWS_FORCE_PATH_STYLE="true"

# Run the service
cd crates/service
cargo run
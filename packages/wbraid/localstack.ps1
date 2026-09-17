# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Stop and remove existing LocalStack containers (if any)
docker ps -q --filter ancestor=localstack/localstack | ForEach-Object { docker stop $_ }
docker ps -aq --filter ancestor=localstack/localstack | ForEach-Object { docker rm $_ }

# Set dummy AWS credentials for LocalStack
$env:AWS_ACCESS_KEY_ID = "test"
$env:AWS_SECRET_ACCESS_KEY = "test"
$env:AWS_DEFAULT_REGION = "us-east-1"

# Start with new configuration
docker run -d -p 4566:4566 -p 4510-4559:4510-4559 `
  -e HOSTNAME_EXTERNAL=localhost `
  -e S3_HOSTNAME=localhost:4566 `
  localstack/localstack

Start-Sleep -Seconds 3.0

# Create bucket and configure CORS (ignore error if bucket already exists)
aws --endpoint-url=http://localhost:4566 s3 mb s3://wbraid-messages 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host "Bucket wbraid-messages already exists or creation failed, continuing..."
}

aws --endpoint-url=http://localhost:4566 s3api put-bucket-cors --bucket wbraid-messages --cors-configuration file://s3-cors.json

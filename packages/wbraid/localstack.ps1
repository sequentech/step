# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Start wbraid's S3 storage on port 4566 and create the wbraid-messages bucket.
# PowerShell twin of localstack.sh.
#
# The storage is MinIO, the image of the devcontainer's `localstack` service,
# with its data in the `wbraid_storage` Docker volume: b4 keeps every message
# body there, and the boards that point at them must still read after a
# restart. `docker volume rm wbraid_storage` removes the bodies.

# The dev-only credentials b4.ps1 gives b4. MinIO refuses a root password
# shorter than 8 characters.
$image = "docker.io/pgsty/silo:RELEASE.2026-09-16T00-00-00Z@sha256:635197cb9f36d01bee221d34d1c7d7960f6a95c48b0b6c01d99cd13bdae51a46"
$env:AWS_ACCESS_KEY_ID = "wbraid-dev"
$env:AWS_SECRET_ACCESS_KEY = "wbraid-dev-storage"
$env:AWS_DEFAULT_REGION = "us-east-1"

# Replace the storage container, and free port 4566 from LocalStack containers
# of earlier runs. The bodies stay in the volume.
docker rm -f wbraid-storage 2>$null | Out-Null
foreach ($old in "localstack/localstack", "localstack/localstack:4") {
    docker ps -aq --filter "ancestor=$old" | ForEach-Object { docker rm -f $_ | Out-Null }
}

docker run -d --name wbraid-storage --restart unless-stopped -p 4566:4566 `
  -e "MINIO_ROOT_USER=$env:AWS_ACCESS_KEY_ID" `
  -e "MINIO_ROOT_PASSWORD=$env:AWS_SECRET_ACCESS_KEY" `
  -v wbraid_storage:/data `
  $image server --address :4566 /data

# Ready in a few seconds warm; a cold start (first pull) outlives a fixed sleep
Write-Host "Waiting for the storage on localhost:4566..."
for ($i = 0; $i -lt 45; $i++) {
    try {
        Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 http://localhost:4566/minio/health/live | Out-Null
        break
    } catch {
        Start-Sleep -Seconds 2
    }
}

# Create the bucket when it is missing. MinIO allows browsers' CORS requests
# from any origin by default and has no per-bucket CORS rules to apply.
aws --endpoint-url=http://localhost:4566 s3api head-bucket --bucket wbraid-messages 2>$null
if ($LASTEXITCODE -ne 0) {
    aws --endpoint-url=http://localhost:4566 s3api create-bucket --bucket wbraid-messages | Out-Null
}

Write-Host "wbraid storage ready: s3://wbraid-messages on localhost:4566."

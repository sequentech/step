# Stop and remove existing LocalStack containers (if any)
docker ps -q --filter ancestor=localstack/localstack | ForEach-Object { docker stop $_ }
docker ps -aq --filter ancestor=localstack/localstack | ForEach-Object { docker rm $_ }

# Start with new configuration
docker run -d -p 4566:4566 -p 4510-4559:4510-4559 `
  -e HOSTNAME_EXTERNAL=localhost `
  -e S3_HOSTNAME=localhost:4566 `
  localstack/localstack

Start-Sleep -Seconds 3.0

# Create bucket and configure CORS
aws --endpoint-url=http://localhost:4566 s3 mb s3://wbraid-messages
aws --endpoint-url=http://localhost:4566 s3api put-bucket-cors --bucket wbraid-messages --cors-configuration file://s3-cors.json

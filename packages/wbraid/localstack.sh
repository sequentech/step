#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Start wbraid's S3 storage on port 4566 and create the wbraid-messages bucket.
# Bash twin of localstack.ps1.
#
# The storage is MinIO with a persistent volume: b4 keeps every message body
# there, and the boards that point at them must still read after a restart.
# The script keeps its name and LocalStack's address (port 4566), which b4,
# the trustees and the docs use.
#
# In the devcontainer, the storage is the `localstack` compose service declared
# in .devcontainer/docker-compose-base.yml (opt-in `wbraid` profile), living on
# the project network like every other dev service; the endpoint there is
# http://localstack:4566, which b4.sh picks up automatically. Outside a compose
# project — a native Linux checkout — this behaves like localstack.ps1: a
# standalone container publishing localhost:4566 with its data in the
# `wbraid_storage` Docker volume.
#
# Restarting keeps the stored bodies. `docker volume rm wbraid_storage` (or
# `docker compose down -v` in the devcontainer) removes them.

set -euo pipefail

cd "$(dirname "$0")"

# The image of the compose service, and the dev-only credentials b4 is given.
# MinIO refuses a root password shorter than 8 characters.
IMAGE="docker.io/pgsty/silo:RELEASE.2026-09-16T00-00-00Z@sha256:635197cb9f36d01bee221d34d1c7d7960f6a95c48b0b6c01d99cd13bdae51a46"
export AWS_ACCESS_KEY_ID="wbraid-dev"
export AWS_SECRET_ACCESS_KEY="wbraid-dev-storage"
export AWS_DEFAULT_REGION="us-east-1"

COMPOSE_PROJECT=$(docker inspect "$(hostname)" \
    --format '{{ index .Config.Labels "com.docker.compose.project" }}' \
    2>/dev/null || true)

if [ -n "$COMPOSE_PROJECT" ]; then
    # The devcontainer: drive the project's own compose services. Targeting a
    # service by name auto-enables its `wbraid` profile; the stored bodies stay
    # in the service's named volume.
    REPO_ROOT=$(cd ../.. && pwd)
    compose() {
        # IGNORE_ORPHANS: containers from older revisions of the compose file
        # may exist in the project; they are not this script's business (and
        # the --remove-orphans hint compose would print must not be followed)
        COMPOSE_IGNORE_ORPHANS=1 docker compose -p "$COMPOSE_PROJECT" \
            -f "$REPO_ROOT/.devcontainer/docker-compose.yml" \
            --env-file "$REPO_ROOT/.devcontainer/.env" "$@"
    }
    compose up -d --wait localstack
    # One-shot bucket provisioning, in the foreground so a failure shows up
    # here
    compose run --rm configure-localstack
    echo "wbraid storage ready at ${WBRAID_S3_ENDPOINT_URL} (s3://wbraid-messages)."
    exit 0
fi

# --- Standalone (no compose project): the localstack.ps1 flow ---------------

if ! command -v aws >/dev/null 2>&1; then
    echo "aws CLI not found on PATH (needed to create the bucket)." >&2
    exit 1
fi

# Replace the storage container, and free port 4566 from LocalStack containers
# of earlier runs. The bodies stay in the volume.
docker rm -f wbraid-storage >/dev/null 2>&1 || true
for image in localstack/localstack localstack/localstack:4; do
    docker ps -aq --filter "ancestor=$image" | xargs -r docker rm -f
done

docker run -d --name wbraid-storage --restart unless-stopped -p 4566:4566 \
    -e MINIO_ROOT_USER="$AWS_ACCESS_KEY_ID" \
    -e MINIO_ROOT_PASSWORD="$AWS_SECRET_ACCESS_KEY" \
    -v wbraid_storage:/data \
    "$IMAGE" server --address :4566 /data

# Ready in a few seconds warm; a cold start (first pull) outlives a fixed sleep
echo "Waiting for the storage on localhost:4566..."
ready=0
for _ in $(seq 1 45); do
    if curl -sf --max-time 2 http://localhost:4566/minio/health/live >/dev/null 2>&1; then
        ready=1
        break
    fi
    sleep 2
done
if [ "$ready" -ne 1 ]; then
    echo "The storage did not become ready on localhost:4566; see 'docker logs wbraid-storage'." >&2
    exit 1
fi

# Create the bucket when it is missing. MinIO allows browsers' CORS requests
# from any origin by default and has no per-bucket CORS rules to apply.
s3() { aws --endpoint-url=http://localhost:4566 s3api "$@"; }
if ! s3 head-bucket --bucket wbraid-messages 2>/dev/null; then
    s3 create-bucket --bucket wbraid-messages >/dev/null
fi

echo "wbraid storage ready: s3://wbraid-messages on localhost:4566."

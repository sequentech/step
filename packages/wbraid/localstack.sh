#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Start a fresh LocalStack and create the wbraid-messages S3 bucket with CORS.
# Bash twin of localstack.ps1.
#
# In the devcontainer, LocalStack is the `localstack` compose service declared
# in .devcontainer/docker-compose-base.yml (opt-in `wbraid` profile), living on
# the project network like every other dev service; the endpoint there is
# http://localstack:4566, which b4.sh picks up automatically. Outside a compose
# project — a native Linux checkout — this behaves like localstack.ps1: a
# standalone container publishing localhost:4566.

set -euo pipefail

cd "$(dirname "$0")"

# Pinned here and in the compose service: from the 2026 calendar releases on,
# localstack/localstack:latest demands an auth token at startup and exits
# without one, so a fresh pull of `latest` breaks. The 4.x line runs
# unauthenticated.
IMAGE="localstack/localstack:4"

COMPOSE_PROJECT=$(docker inspect "$(hostname)" \
    --format '{{ index .Config.Labels "com.docker.compose.project" }}' \
    2>/dev/null || true)

if [ -n "$COMPOSE_PROJECT" ]; then
    # The devcontainer: drive the project's own compose services. Targeting a
    # service by name auto-enables its `wbraid` profile, and --force-recreate
    # gives the same start-fresh semantic as the standalone path below. The
    # config's bind mounts need host paths, which .devcontainer/.env carries
    # (LOCAL_WORKSPACE_FOLDER).
    REPO_ROOT=$(cd ../.. && pwd)
    compose() {
        # IGNORE_ORPHANS: containers from older revisions of the compose file
        # may exist in the project; they are not this script's business (and
        # the --remove-orphans hint compose would print must not be followed)
        COMPOSE_IGNORE_ORPHANS=1 docker compose -p "$COMPOSE_PROJECT" \
            -f "$REPO_ROOT/.devcontainer/docker-compose.yml" \
            --env-file "$REPO_ROOT/.devcontainer/.env" "$@"
    }
    compose up -d --force-recreate localstack
    # One-shot bucket + CORS provisioning, in the foreground so a failure
    # shows up here
    compose run --rm configure-localstack
    echo "LocalStack ready at http://localstack:4566 (s3://wbraid-messages with CORS applied)."
    exit 0
fi

# --- Standalone (no compose project): the localstack.ps1 flow ---------------

if ! command -v aws >/dev/null 2>&1; then
    echo "aws CLI not found on PATH (needed to create the bucket and apply CORS)." >&2
    exit 1
fi

# Stop and remove existing LocalStack containers (if any); the bare image name
# also catches :latest strays from earlier runs
for image in localstack/localstack "$IMAGE"; do
    docker ps -q --filter "ancestor=$image" | xargs -r docker stop
    docker ps -aq --filter "ancestor=$image" | xargs -r docker rm
done

# Set dummy AWS credentials for LocalStack
export AWS_ACCESS_KEY_ID="test"
export AWS_SECRET_ACCESS_KEY="test"
export AWS_DEFAULT_REGION="us-east-1"

# Start with new configuration
docker run -d -p 4566:4566 -p 4510-4559:4510-4559 \
    -e HOSTNAME_EXTERNAL=localhost \
    -e S3_HOSTNAME=localhost:4566 \
    "$IMAGE"

# Ready in a few seconds warm; a cold start (first pull) outlives a fixed sleep
echo "Waiting for LocalStack on localhost:4566..."
for _ in $(seq 1 45); do
    if curl -sf --max-time 2 http://localhost:4566/_localstack/health >/dev/null 2>&1; then
        break
    fi
    sleep 2
done

# Create bucket and configure CORS (ignore error if bucket already exists)
if ! aws --endpoint-url=http://localhost:4566 s3 mb s3://wbraid-messages 2>/dev/null; then
    echo "Bucket wbraid-messages already exists or creation failed, continuing..."
fi

aws --endpoint-url=http://localhost:4566 s3api put-bucket-cors \
    --bucket wbraid-messages --cors-configuration file://s3-cors.json

echo "LocalStack ready: s3://wbraid-messages exists with CORS applied."

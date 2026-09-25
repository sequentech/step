#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# Start the devcontainer's Hasura image on a fresh PostgreSQL 18 database, let
# it apply every migration and the metadata, and require consistent metadata.
# The containers use a private network and publish no ports.
#
# Usage: scripts/postgres/check_hasura_metadata.sh

set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
hasura=$root/hasura
name=hasura-metadata-check-$(python3 -c 'import uuid; print(uuid.uuid4().hex)')
database=postgres://postgres:postgres@postgres:5432/postgres
secret='admin'
network_id= postgres_id= hasura_id=

for directory in "$hasura/migrations/backend-db" "$hasura/metadata"; do
    [ -d "$directory" ] || { echo "$directory does not exist" >&2; exit 1; }
done

cleanup() {
    # Names can be reused after removal; only successful creates confer ownership.
    for container_id in "$hasura_id" "$postgres_id"; do
        if [ -n "$container_id" ]; then
            docker rm --force --volumes "$container_id" >/dev/null 2>&1 || true
        fi
    done
    if [ -n "$network_id" ]; then
        docker network rm "$network_id" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

created_id=$(docker network create "$name")
network_id=$created_id
# Capture ownership before start, which can fail after Docker has created a container.
created_id=$(docker create --name "$name-postgres" --network "$network_id" --network-alias postgres \
    --env POSTGRES_PASSWORD=postgres \
    --volume "$root/.devcontainer/postgresql/init.sh:/docker-entrypoint-initdb.d/init.sh:ro" \
    postgres:18-bookworm)
postgres_id=$created_id
docker start "$postgres_id" >/dev/null
# The image only listens on TCP once its initialization scripts have finished.
ready=false
for _ in $(seq 60); do
    if docker exec "$postgres_id" pg_isready --host=127.0.0.1 --quiet; then
        ready=true
        break
    fi
    sleep 1
done
if [ "$ready" != true ]; then
    docker logs "$postgres_id" 2>&1 | tail -n 20 >&2
    echo "PostgreSQL did not start" >&2
    exit 1
fi

# The actions read HARVEST_DOMAIN and ACTIONS_ADMIN_SECRET; without them the
# metadata is inconsistent. The values follow .devcontainer/.env.development.
created_id=$(docker create --name "$name-hasura" --network "$network_id" \
    --volume "$hasura/migrations:/hasura-migrations:ro" \
    --volume "$hasura/metadata:/hasura-metadata:ro" \
    --env HASURA_GRAPHQL_METADATA_DATABASE_URL="$database" \
    --env PG_DATABASE_URL="$database" \
    --env HASURA_GRAPHQL_ADMIN_SECRET="$secret" \
    --env ACTIONS_ADMIN_SECRET="$secret" \
    --env HARVEST_DOMAIN=harvest:8400 \
    --env HASURA_GRAPHQL_ENABLE_TELEMETRY=false \
    hasura/graphql-engine:v2.33.1.cli-migrations-v3)
hasura_id=$created_id
docker start "$hasura_id" >/dev/null

# The entrypoint exits if a migration or the metadata cannot be applied.
healthy=false
for _ in $(seq 120); do
    if [ "$(docker inspect --format '{{.State.Running}}' "$hasura_id")" != true ]; then
        break
    fi
    if docker exec "$hasura_id" curl --silent --fail http://localhost:8080/healthz >/dev/null; then
        healthy=true
        break
    fi
    sleep 1
done
if [ "$healthy" != true ]; then
    docker logs "$hasura_id" 2>&1 | tail -n 50 >&2
    echo "Hasura did not start" >&2
    exit 1
fi

expected=$(find "$hasura/migrations/backend-db" -mindepth 1 -maxdepth 1 -type d | wc -l)
applied=$(docker exec "$postgres_id" psql --username=postgres --no-psqlrc --tuples-only --no-align \
    --command="SELECT count(*) FROM jsonb_object_keys(
        (SELECT cli_state -> 'migrations' -> 'backend-db' FROM hdb_catalog.hdb_version))")
if [ "$applied" -ne "$expected" ]; then
    echo "Hasura recorded $applied of $expected migrations" >&2
    exit 1
fi

docker exec "$hasura_id" curl --silent --fail \
    --header "X-Hasura-Admin-Secret: $secret" \
    --data '{"type": "get_inconsistent_metadata", "args": {}}' \
    http://localhost:8080/v1/metadata | python3 -c '
import json
import sys

report = json.load(sys.stdin)
for item in report["inconsistent_objects"]:
    print(item.get("name", item["type"]), "-", item.get("reason"), file=sys.stderr)
if report["is_consistent"] is not True:
    sys.exit("Hasura metadata is inconsistent")
'
echo "Hasura applied $applied migrations and reports consistent metadata"

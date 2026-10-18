#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# Integration test for the Hasura image entrypoint (meta#14308): a pod start
# with an already-applied image must not re-apply metadata or migrations.
# Needs Docker. Run from the repo root: hasura/tests/entrypoint.test.sh
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
TAG=local/sequent-hasura-entrypoint-test
NET=hasura-entrypoint-test
PG=$NET-pg
HGE=$NET-hge
URL=postgres://postgres:pw@$PG:5432/postgres
fails=0

cleanup() { docker rm -f "$HGE" "$PG" >/dev/null 2>&1 || true; docker network rm "$NET" >/dev/null 2>&1 || true; }
trap cleanup EXIT

build() { docker build -q --platform linux/amd64 -f "$ROOT/packages/Dockerfile.hasura" -t "$1" "$2" >/dev/null; }
sql() { docker exec "$PG" psql -U postgres -tAc "$1" 2>/dev/null || true; }
rv() { sql 'SELECT resource_version FROM hdb_catalog.hdb_metadata'; }

# start <image>: runs the image the way the clusters do (image ENTRYPOINT,
# args = the server command) and waits for /healthz.
start() {
  docker rm -f "$HGE" >/dev/null 2>&1 || true
  docker run -d --name "$HGE" --network "$NET" --platform linux/amd64 \
    -e HASURA_GRAPHQL_DATABASE_URL="$URL" -e PG_DATABASE_URL="$URL" -e HARVEST_DOMAIN=harvest:8400 \
    "$1" graphql-engine serve >/dev/null
  for _ in $(seq 1 180); do
    docker exec "$HGE" curl -sf localhost:8080/healthz >/dev/null 2>&1 && return 0
    [ "$(docker inspect -f '{{.State.Running}}' "$HGE")" = true ] || break
    sleep 1
  done
  echo "  hasura did not become healthy:"; docker logs "$HGE" 2>&1 | tail -5
  return 1
}

check() { # check <name> <expected> <actual>
  if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: expected '$2', got '$3'"; fails=$((fails + 1)); fi
}
applied() { docker logs "$HGE" 2>&1 | grep -c '"info":"applying metadata from' || true; }

docker network create "$NET" >/dev/null
docker run -d --name "$PG" --network "$NET" -e POSTGRES_PASSWORD=pw postgres:15 >/dev/null
for _ in $(seq 1 30); do docker exec "$PG" pg_isready -U postgres >/dev/null 2>&1 && break; sleep 1; done
sleep 2

build "$TAG:a" "$ROOT/hasura"

start "$TAG:a"
check "fresh DB: applies metadata" 1 "$(applied)"
first=$(rv)

start "$TAG:a"
check "same image restarted: skips the apply" 0 "$(applied)"
check "same image restarted: resource version unchanged" "$first" "$(rv)"

start "$TAG:a"
check "same image, third start: resource version unchanged" "$first" "$(rv)"

# A new image whose metadata differs must be applied once, then skipped.
CHANGED=$(mktemp -d)
cp -a "$ROOT/hasura/." "$CHANGED/"
printf -- '- name: entrypoint_test\n  definition:\n    queries: []\n' > "$CHANGED/metadata/query_collections.yaml"
build "$TAG:b" "$CHANGED"
rm -rf "$CHANGED"

start "$TAG:b"
check "changed metadata: applies" 1 "$(applied)"
check "changed metadata: is live" 1 "$(sql "SELECT count(*) FROM hdb_catalog.hdb_metadata WHERE metadata::text LIKE '%entrypoint_test%'")"
second=$(rv)
start "$TAG:b"
check "changed image restarted: skips the apply" 0 "$(applied)"
check "changed image restarted: resource version unchanged" "$second" "$(rv)"

# Without a stored state the image must fall back to applying.
sql 'DELETE FROM sequent_ops.hasura_apply_state' >/dev/null
start "$TAG:b"
check "state row missing: applies" 1 "$(applied)"

[ "$fails" -eq 0 ] && echo "all passed" || { echo "$fails failed"; exit 1; }

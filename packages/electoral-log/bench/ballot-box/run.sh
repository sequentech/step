#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Measures the accept path of the log-based ballot box, alone and together with
# Merkle-log appends of the kind the sequencer makes. Run from the repository
# root on the development host, with the development stack up. It starts its own
# PostgreSQL container and removes it, with its data, at the end.
#
#   packages/electoral-log/bench/ballot-box/run.sh [seconds per run]
set -euo pipefail

SECONDS_PER_RUN="${1:-20}"
HERE="packages/electoral-log/bench/ballot-box"
CONTAINER=elog-ballot-box-bench
VOLUME=elog-ballot-box-bench-data
NETWORK=step_devcontainer_default
IMAGE=sequentech.local/postgresql:latest
CLIENT=postgres # a running container with pgbench on the same network

cleanup() {
    docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
    docker volume rm "$VOLUME" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# Same tuning as the electoral-log load test, scaled to 4 CPUs and 6 GB.
docker run -d --name "$CONTAINER" --network "$NETWORK" --cpus 4 --memory 6g --shm-size 1g \
    -e POSTGRES_PASSWORD=bench -e POSTGRES_DB=ballot_box -v "$VOLUME":/var/lib/postgresql "$IMAGE" \
    postgres -c shared_buffers=1536MB -c effective_cache_size=4GB -c work_mem=32MB \
    -c maintenance_work_mem=512MB -c max_wal_size=8GB -c random_page_cost=1.1 \
    -c effective_io_concurrency=200 -c synchronous_commit=on -c max_connections=200 \
    -c wal_compression=lz4 >/dev/null
until docker exec "$CONTAINER" pg_isready -U postgres -d ballot_box >/dev/null 2>&1; do sleep 1; done

psql() { docker exec -i "$CONTAINER" psql -U postgres -d ballot_box -q -v ON_ERROR_STOP=1 "$@"; }
psql < "$HERE/schema.sql"
psql < packages/electoral-log/schema.sql
psql -c "CREATE FUNCTION bench_content(bytes integer) RETURNS bytea IMMUTABLE LANGUAGE sql
         AS \$\$ SELECT decode(repeat('ab', bytes), 'hex') \$\$;"

accept() { # clients bytes
    sed "s/@BYTES@/$2/" "$HERE/accept.sql" | docker exec -i "$CLIENT" sh -c "cat > /tmp/ballot-box-accept.sql"
    docker exec -e PGPASSWORD=bench "$CLIENT" pgbench -h "$CONTAINER" -U postgres -n -M prepared \
        -c "$1" -j 4 -T "$SECONDS_PER_RUN" -D voters=1000000 -D elections=5 -D max_votes=3 \
        -f /tmp/ballot-box-accept.sql ballot_box 2>&1 |
        grep -E "number of clients|processed|failed|latency average|tps" | tr "\n" " "
    echo "bytes=$2"
}

for bytes in 2048 5120; do
    for clients in 16 32 64; do
        accept "$clients" "$bytes"
        psql -c "TRUNCATE ballot, voter_state; CHECKPOINT;"
    done
done

# Accept and sequencer together: Merkle-log appends of 10,000 records, through the
# load-test client, while 32 clients accept 2 KB ballots.
accept 32 2048 > /tmp/ballot-box-combined.txt &
sleep 3
docker exec devcontainer bash -c "cd /workspaces/step/packages/electoral-log && \
    ELECTORAL_LOG_PG_HOST=$CONTAINER ELECTORAL_LOG_PG_PORT=5432 ELECTORAL_LOG_PG_USER=postgres \
    ELECTORAL_LOG_PG_PASSWORD=bench ELECTORAL_LOG_PG_DATABASE=ballot_box ELECTORAL_LOG_PG_SSLMODE=disable \
    timeout $SECONDS_PER_RUN rust-local-target/release/examples/load_test \
    --checkpoints /tmp/ballot-box-checkpoints.jsonl fill --target 10000000 --batch 10000 --report 100000" || true
wait
echo "accept while appending: $(cat /tmp/ballot-box-combined.txt)"

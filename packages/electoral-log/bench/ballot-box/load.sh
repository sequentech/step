#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Load test of the ballot box as implemented: votes accepted through
# `PostgresStore::accept_ballot` for minutes at a time, ballot boxes created and
# dropped while votes go on, the reads of an event with over a million ballots, and
# the sequencer's appends. Run from the repository root on the development host,
# with the development stack up and the load tool built in the devcontainer:
#
#   (devcontainer, packages/electoral-log)
#   cargo build --release --target-dir rust-local-target --example ballot_box_load
#   packages/electoral-log/bench/ballot-box/load.sh [seconds per run] [scenarios]
#
# Scenarios: accept, reads, ddl and sequencer; all of them by default. It starts its
# own PostgreSQL container, tuned as run.sh's, and removes it, with its data, at the
# end. The longest scenario writes about 6 GB.
set -euo pipefail

SECONDS_PER_RUN="${1:-120}"
SCENARIOS="${2:-accept reads ddl sequencer}"
SHORT_RUN=$((SECONDS_PER_RUN * 3 / 4))
CONTAINER=elog-ballot-box-load
VOLUME=elog-ballot-box-load-data
NETWORK=step_devcontainer_default
IMAGE=sequentech.local/postgresql:latest

cleanup() {
    docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
    docker volume rm "$VOLUME" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run -d --name "$CONTAINER" --network "$NETWORK" --cpus 4 --memory 6g --shm-size 1g \
    -e POSTGRES_PASSWORD=bench -e POSTGRES_DB=ballot_box -v "$VOLUME":/var/lib/postgresql "$IMAGE" \
    postgres -c shared_buffers=1536MB -c effective_cache_size=4GB -c work_mem=32MB \
    -c maintenance_work_mem=512MB -c max_wal_size=4GB -c random_page_cost=1.1 \
    -c effective_io_concurrency=200 -c synchronous_commit=on -c max_connections=200 \
    -c wal_compression=lz4 -c log_checkpoints=on -c log_autovacuum_min_duration=0 >/dev/null
until docker exec "$CONTAINER" pg_isready -U postgres -d ballot_box >/dev/null 2>&1; do sleep 1; done
sleep 2

psql() { docker exec -i "$CONTAINER" psql -U postgres -d ballot_box -q -v ON_ERROR_STOP=1 "$@"; }
psql < packages/electoral-log/schema.sql

tool() {
    docker exec devcontainer bash -c "cd /workspaces/step/packages/electoral-log && \
        ELECTORAL_LOG_PG_HOST=$CONTAINER ELECTORAL_LOG_PG_PORT=5432 ELECTORAL_LOG_PG_USER=postgres \
        ELECTORAL_LOG_PG_PASSWORD=bench ELECTORAL_LOG_PG_DATABASE=ballot_box ELECTORAL_LOG_PG_SSLMODE=disable \
        rust-local-target/release/examples/ballot_box_load $*"
}
wanted() { [[ " $SCENARIOS " == *" $1 "* ]]; }
reset() { psql -c "TRUNCATE ballot_box_ballot, ballot_box_voter, ballot_box_pending;" -c "CHECKPOINT;"; }
event_of() { grep -o '^event [0-9a-f-]*' "$1" | cut -d' ' -f2; }
activity() { # what the server did since the given time
    local logs
    logs=$(docker logs --since "$1" "$CONTAINER" 2>&1)
    echo "checkpoints: $(grep -c 'checkpoint complete' <<<"$logs" || true)"
    echo "autovacuum and autoanalyze runs: $(grep -cE 'automatic (vacuum|analyze)' <<<"$logs" || true)"
}

if wanted accept || wanted reads; then
    echo "== 2 KB ballots, 32 clients, $SECONDS_PER_RUN s"
    START=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    tool accept --seconds "$SECONDS_PER_RUN" --clients 32 --bytes 2048 | tee /tmp/ballot-box-load-2k.txt
    activity "$START"
    EVENT=$(event_of /tmp/ballot-box-load-2k.txt)
    psql -c "SELECT pg_size_pretty(pg_database_size('ballot_box')) AS database"

    echo "== Correctness of the run"
    psql <<SQL
SELECT count(*) AS voters_over_the_limit FROM ballot_box_voter WHERE votes > 3;
SELECT count(*) AS voters_whose_count_differs_from_their_ballots
FROM ballot_box_voter v
JOIN (SELECT election_event_id, election_id, voter_id, count(*) AS ballots
      FROM ballot_box_ballot GROUP BY 1, 2, 3) b USING (election_event_id, election_id, voter_id)
WHERE b.ballots <> v.votes;
SELECT (SELECT count(*) FROM ballot_box_ballot) AS ballots,
       (SELECT sum(votes) FROM ballot_box_voter) AS counted_votes,
       (SELECT count(*) FROM ballot_box_pending) AS queued;
SQL

    if wanted reads; then
        echo "== Reads of the event, once the sequencer would have appended every ballot"
        tool settle --event "$EVENT"
        psql -c "VACUUM ANALYZE ballot_box_ballot;" -c "VACUUM ANALYZE ballot_box_voter;"
        tool reads --event "$EVENT" --voters 1000000
    fi
    reset
fi

if wanted accept; then
    echo "== 5 KB ballots, 32 clients, $SHORT_RUN s"
    START=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    tool accept --seconds "$SHORT_RUN" --clients 32 --bytes 5120
    activity "$START"
    reset

    echo "== 2 KB ballots, 64 clients, $SHORT_RUN s"
    START=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    tool accept --seconds "$SHORT_RUN" --clients 64 --bytes 2048
    activity "$START"
    reset
fi

if wanted ddl; then
    echo "== 5,000 votes/s, 2 KB ballots, without and with a ballot box created and dropped every 10 s"
    tool accept --seconds "$SHORT_RUN" --clients 32 --bytes 2048 --rate 5000
    reset
    tool accept --seconds "$SHORT_RUN" --clients 32 --bytes 2048 --rate 5000 --churn-every 10
    reset
fi

if wanted sequencer; then
    echo "== The sequencer alone, appending a backlog of 2 KB ballots"
    tool accept --seconds 30 --clients 32 --bytes 2048 | tee /tmp/ballot-box-load-backlog.txt | tail -1
    EVENT=$(event_of /tmp/ballot-box-load-backlog.txt)
    tool sequence --event "$EVENT"

    echo "== The sequencer while 32 clients accept 2 KB ballots as fast as they can, then catching up"
    tool sequence --event "$EVENT" --linger 15 > /tmp/ballot-box-load-sequencer.txt &
    SEQUENCER=$!
    tool accept --event "$EVENT" --seconds "$SHORT_RUN" --clients 32 --bytes 2048 | tail -1
    wait "$SEQUENCER"
    cat /tmp/ballot-box-load-sequencer.txt

    echo "== The sequencer while votes arrive at 5,000/s"
    tool sequence --event "$EVENT" --linger 15 > /tmp/ballot-box-load-sequencer.txt &
    SEQUENCER=$!
    tool accept --event "$EVENT" --seconds "$SHORT_RUN" --clients 32 --bytes 2048 --rate 5000 | tail -1
    wait "$SEQUENCER"
    cat /tmp/ballot-box-load-sequencer.txt
    reset
fi

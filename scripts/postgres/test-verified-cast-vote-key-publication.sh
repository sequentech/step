#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Runs only against a fresh Unix-socket PostgreSQL fixture, never an existing
# server. Point PG_BIN at a complete local PostgreSQL installation.
set -euo pipefail
umask 077
root=$(cd "$(dirname "$0")/../.." && pwd)
fixtures="$root/hasura/tests"
migrations="$root/hasura/migrations/backend-db"
work=$(mktemp -d "/tmp/verified-cast-key-test.XXXXXXXX")
pg_bin="${PG_BIN:-$(dirname "$(command -v psql)")}"
finish() {
    "$pg_bin/pg_ctl" -D "$work/db" -m immediate stop >/dev/null 2>&1 || true
    rm -rf -- "$work"
}
trap finish EXIT
mkdir "$work/socket"

# Drop ambient connection parameters before invoking the test client.
unset PGSERVICE PGSERVICEFILE PGDATABASE PGUSER PGPASSWORD PGOPTIONS PGHOSTADDR
export PGHOST="$work/socket" PGPORT=55483
"$pg_bin/initdb" --auth=trust --no-locale -D "$work/db" >/dev/null
if ! "$pg_bin/pg_ctl" -D "$work/db" -w start -l "$work/postgres.log" \
    -o "-c listen_addresses='' -k $work/socket -p $PGPORT -c shared_buffers=32MB -c fsync=off" >/dev/null; then
    tail -n 20 "$work/postgres.log" >&2
    exit 1
fi
sql() { "$pg_bin/psql" -Xq -v ON_ERROR_STOP=1 -d postgres "$@"; }

# The table/role subset used by the three real predecessor migrations.
sql <<'SQL'
CREATE SCHEMA sequent_backend;
CREATE ROLE sequent_os_ledger_data_owner NOLOGIN;
CREATE ROLE sequent_os_ledger_api_owner NOLOGIN;
GRANT USAGE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner, sequent_os_ledger_data_owner;
CREATE TABLE sequent_backend.tenant(id uuid PRIMARY KEY);
CREATE TABLE sequent_backend.election_event (
    tenant_id uuid NOT NULL REFERENCES sequent_backend.tenant, id uuid NOT NULL,
    status jsonb, PRIMARY KEY (tenant_id,id)
);
CREATE TABLE sequent_backend.election (
    tenant_id uuid NOT NULL, election_event_id uuid NOT NULL, id uuid PRIMARY KEY, status jsonb,
    FOREIGN KEY (tenant_id,election_event_id) REFERENCES sequent_backend.election_event
);
GRANT SELECT ON sequent_backend.election_event, sequent_backend.election TO sequent_os_ledger_api_owner;
SQL
for name in 1790640000600_configuration_package 1794700000600_managed_webauthn_roster 1794700001600_cast_vote_seal_key; do
    sql -f "$migrations/$name/up.sql"
done
publication="$migrations/1795658400100_verified_cast_vote_key_publication"
sql -f "$publication/up.sql"
sql -f "$publication/down.sql"
sql -f "$publication/up.sql"
sql -f "$fixtures/verified-cast-vote-key-publication.test.sql"

# Both contenders address the same pending receipt; exactly one distinct
# point wins, and no eventual rekey or second receipt is possible.
race() {
    sql -A -t -v point="$1" <<'SQL'
SET SESSION AUTHORIZATION synthetic_cast_publisher;
SELECT sequent_backend.verified_cast_key_stage(
    job_id::uuid, tenant_id::uuid, election_event_id::uuid, manifest_sha256, decode(:'point','hex'))
FROM sequent_backend.verified_cast_key_next();
SQL
}
# The genuine P-256 generator at either parity is sufficient: the worker
# independently validates the full curve point before staging.
x=6b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296
y=4fe342e2fe1a7f9b8ee7eb4a7c0f9e162bce33576b315ececbb6406837bf51f5
negative_y=b01cbd1c01e58065711814b583f061e9d431cca994cea1313449bf97c840ae0a
# Both sessions take simultaneous snapshots before entering the stage
# function. A ready-barrier is represented by the same advisory lock.
# The separate SQL fixture also tests a sequential retry of the exact point.
sql -At <<'SQL' >"$work/hold.log" &
BEGIN;
SELECT sequent_backend.cast_vote_seal_lock(
    '11111111-1111-4111-8111-111111111111',
    '44444444-4444-4444-8444-444444444444'
);
SELECT pg_sleep(1);
COMMIT;
SQL
holder=$!
for _ in $(seq 1 100); do
    locks=$(sql -Atc "SELECT count(*) FROM pg_locks WHERE locktype='advisory' AND granted")
    if [[ "$locks" == 1 ]]; then break; fi
    sleep .02
done
[[ "$locks" == 1 ]]
race "04$x$y" >"$work/a" &
a=$!
race "04$x$negative_y" >"$work/b" &
b=$!
wait "$holder" "$a" "$b"
actual=$(cat "$work/a" "$work/b" | sort | paste -sd, -)
[[ "$actual" == f,t ]]
[[ "$(sql -Atc "SELECT count(*) FROM sequent_backend.verified_cast_key_job WHERE public_key_sec1 IS NOT NULL")" == 2 ]]
if sql -f "$publication/down.sql" >"$work/blocked-down.log" 2>&1; then
    echo "The migration allowed downgrade with authorized jobs" >&2
    exit 1
fi
grep -q 'cannot downgrade verified cast key publication' "$work/blocked-down.log"
printf 'Verified scoped cast-key migration, ACL, pre-open and race regressions passed\n'

#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# Apply every Hasura backend-db migration to an empty database in version order,
# roll back all but the oldest in reverse order, require the baseline schema,
# then reapply and require the fully migrated schema. Each file runs in its own
# transaction. The oldest migration is the squashed baseline: its generated
# down.sql is not executable, and rolling it back would drop the whole schema.
#
# The disposable cluster listens only on a private socket, so no running
# database is touched. Run it as a non-root user with the PostgreSQL 18 server
# binaries on PATH: inside devenv, or in the postgres:18-bookworm image:
#
#   docker run --rm --user postgres --volume "$PWD:/step:ro" --workdir /step \
#     postgres:18-bookworm scripts/postgres/check_migrations.sh
#
# Usage: scripts/postgres/check_migrations.sh [migrations-directory]

set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
migrations=${1:-$root/hasura/migrations/backend-db}

if [ "$(id -u)" -eq 0 ]; then
    echo "initdb refuses to run as root; use a non-root user such as postgres" >&2
    exit 1
fi

versions=()
while IFS= read -r version; do
    for direction in up down; do
        if [ ! -f "$migrations/$version/$direction.sql" ]; then
            echo "$migrations/$version has no $direction.sql" >&2
            exit 1
        fi
    done
    versions+=("$version")
done < <(find "$migrations" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort -t_ -k1,1n)
if [ "${#versions[@]}" -eq 0 ]; then
    echo "No migrations found in $migrations" >&2
    exit 1
fi

work=$(mktemp -d "${TMPDIR:-/tmp}/check-migrations.XXXXXX")
cleanup() {
    pg_ctl --pgdata="$work/data" --mode=immediate --wait stop >/dev/null 2>&1 || true
    rm -rf "$work"
}
trap cleanup EXIT

initdb --pgdata="$work/data" --auth=trust --username=postgres --encoding=UTF8 >/dev/null
pg_ctl --pgdata="$work/data" --log="$work/postgres.log" --wait \
    --options="-c listen_addresses='' -k $work" start >/dev/null

export PGHOST=$work PGUSER=postgres PGDATABASE=postgres
export PGOPTIONS='-c client_min_messages=warning'
run() { psql --no-psqlrc --quiet --set=ON_ERROR_STOP=1 "$@" >/dev/null; }
apply() {
    local direction=$1 version
    shift
    for version in "$@"; do
        run --single-transaction --file="$migrations/$version/$direction.sql"
    done
}
# Ignore dump nonce and physical column order, which DROP/ADD cannot restore.
schema() {
    pg_dump --schema-only --no-owner --no-privileges \
        | awk -f "$root/scripts/postgres/normalize_schema.awk" >"$1"
}

# The extensions that .devcontainer/postgresql/init.sh creates.
run --command='CREATE EXTENSION IF NOT EXISTS pgcrypto' \
    --command='CREATE EXTENSION IF NOT EXISTS unaccent'

baseline=${versions[0]}
reversible=("${versions[@]:1}")
apply up "$baseline"
schema "$work/baseline.sql"
apply up "${reversible[@]}"
schema "$work/applied.sql"
echo "Applied ${#versions[@]} migrations"

reversed=()
for ((index = ${#reversible[@]} - 1; index >= 0; index--)); do
    reversed+=("${reversible[index]}")
done
apply down "${reversed[@]}"
schema "$work/rolled-back.sql"
diff -u "$work/baseline.sql" "$work/rolled-back.sql" >&2
echo "Rolled back ${#reversible[@]} migrations to the ${baseline%%_*} baseline"

apply up "${reversible[@]}"
schema "$work/reapplied.sql"
diff -u "$work/applied.sql" "$work/reapplied.sql" >&2
echo "Reapplied ${#reversible[@]} migrations with an identical schema"

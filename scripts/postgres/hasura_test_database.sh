#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# Turn an empty, disposable database into a Hasura test database: create the
# extensions that .devcontainer/postgresql/init.sh creates, then apply every
# Hasura backend-db migration's up.sql in numeric version order, each in its
# own transaction, stopping at the first error. Code reading the Hasura
# database through HASURA_DB__* or HASURA_TEST_DATABASE_URL then finds the
# full schema (the electoral-log outbox, tenants, the vault's MAC keys...).
#
# Only point it at a test service's database, such as the CI job's postgres
# service. A database that already has the sequent_backend schema is refused
# before anything is changed. psql must be on PATH (any version that speaks to
# the server).
#
#   scripts/postgres/hasura_test_database.sh postgres://test:test@127.0.0.1:3322/test
#
# Usage: scripts/postgres/hasura_test_database.sh database-url [migrations-directory]

set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
url=${1:?usage: $0 database-url [migrations-directory]}
migrations=${2:-$root/hasura/migrations/backend-db}

versions=()
while IFS= read -r version; do
    if [ ! -f "$migrations/$version/up.sql" ]; then
        echo "$migrations/$version has no up.sql" >&2
        exit 1
    fi
    versions+=("$version")
done < <(find "$migrations" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort -t_ -k1,1n)
if [ "${#versions[@]}" -eq 0 ]; then
    echo "No migrations found in $migrations" >&2
    exit 1
fi

# The squashed baseline holds its own BEGIN/COMMIT; only errors matter.
export PGOPTIONS='-c client_min_messages=error'
run() { psql --no-psqlrc --quiet --set=ON_ERROR_STOP=1 --dbname="$url" "$@" >/dev/null; }

existing=$(psql --no-psqlrc --quiet --set=ON_ERROR_STOP=1 --dbname="$url" \
    --tuples-only --no-align \
    --command="SELECT count(*) FROM pg_namespace WHERE nspname = 'sequent_backend'")
if [ "$existing" != 0 ]; then
    echo "The database already has the sequent_backend schema; use an empty one" >&2
    exit 1
fi
run --command='CREATE EXTENSION IF NOT EXISTS pgcrypto' \
    --command='CREATE EXTENSION IF NOT EXISTS unaccent'
for version in "${versions[@]}"; do
    run --single-transaction --file="$migrations/$version/up.sql"
done
echo "Applied ${#versions[@]} Hasura migrations"

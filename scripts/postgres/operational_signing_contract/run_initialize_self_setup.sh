#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
#
# Exercise the real migration chain's first-use assertions on one disposable,
# socket-only PostgreSQL 17+ database. No existing database is contacted.
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
migrations="$here/../../../hasura/migrations/backend-db"
scratch=$(mktemp -d "${TMPDIR:-/tmp}/initialize-self-setup.XXXXXX")
# Keep Unix socket paths short enough on macOS.
scratch=$(cd "$scratch" && pwd -P)
bindir=${PGBINDIR:-$(pg_config --bindir)}
dbport=56489
unset PGHOST PGHOSTADDR PGDATABASE PGUSER PGPORT PGPASSWORD PGSERVICE PGSERVICEFILE PGOPTIONS PGAPPNAME
export PGHOST="$scratch" PGPORT="$dbport" PGDATABASE=postgres LC_ALL=C
started=false
cleanup() {
    if "$started"; then
        "$bindir/pg_ctl" -D "$scratch/pg" -m fast -w stop >/dev/null
    fi
}
trap cleanup EXIT
"$bindir/initdb" -D "$scratch/pg" -A reject --auth-local=trust --no-locale >"$scratch/init.log"
"$bindir/pg_ctl" -D "$scratch/pg" -l "$scratch/server.log" \
    -o "-k $scratch -c listen_addresses='' -p $dbport" -w start >/dev/null
started=true
sql() { "$bindir/psql" -X -q -v ON_ERROR_STOP=1 "$@"; }
export PGOPTIONS='-c client_min_messages=warning'
sql -c 'CREATE EXTENSION pgcrypto; CREATE EXTENSION unaccent'
for migration in "$migrations"/*; do
    version=${migration##*/}; number=${version%%_*}
    [ "$number" -le 1794700001700 ] || break
    sql --single-transaction --file="$migration/up.sql" >"$scratch/migration.log" 2>&1 || {
        echo "Migration failed: $version" >&2; cat "$scratch/migration.log" >&2; exit 1;
    }
done
sql -f "$here/10_fixture.sql" >"$scratch/fixture.log" 2>&1 || {
    cat "$scratch/fixture.log" >&2; exit 1;
}
PGOPTIONS='' sql -f "$here/25_initialize_self_setup.sql" >"$scratch/result.log" 2>&1 || {
    cat "$scratch/result.log" >&2; exit 1;
}
grep 'NOTICE:  ok' "$scratch/result.log"
echo "Synthetic first-use signing assertions passed; disposable logs: $scratch"

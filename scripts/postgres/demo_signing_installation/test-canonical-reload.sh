#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
# Create a dedicated socket-only database, using no supplied connection.
set -eu
umask 077
unset PGHOST PGHOSTADDR PGPORT PGUSER PGDATABASE PGPASSWORD PGSERVICE PGSERVICEFILE PGOPTIONS
root=$(cd "$(dirname "$0")/../../.." && pwd)
pg=${PG_BINDIR:-$(pg_config --bindir)}
p=$(mktemp -d "${TMPDIR:-/tmp}/demo-canonical.XXXXXXXX")
p=$(cd "$p" && pwd -P)
export PGHOST="$p" PGPORT=46337 PGUSER=canonical_fixture PGDATABASE=postgres LC_ALL=C
"$pg/initdb" -D "$p/pg" -U "$PGUSER" -A reject --auth-local=trust --no-locale >"$p/init.log"
"$pg/pg_ctl" -D "$p/pg" -l "$p/log" -o "-k '$p' -h '' -p $PGPORT" -w start >/dev/null
trap '"$pg/pg_ctl" -D "$p/pg" -m fast -w stop >/dev/null; rm -rf -- "$p"' EXIT
"$pg/createdb" canonical_request_14990
export PGDATABASE=canonical_request_14990
{ echo 'CREATE EXTENSION pgcrypto; CREATE EXTENSION unaccent;'
  for d in "$root"/hasura/migrations/backend-db/*; do echo 'BEGIN;'; cat "$d/up.sql"; echo; echo 'COMMIT;'; done
} >"$p/migrations.sql"
"$pg/psql" -X -q -v ON_ERROR_STOP=1 -f "$p/migrations.sql" >"$p/migrations.log" 2>&1 || { tail -6 "$p/migrations.log"; exit 1; }
export SIGNING_CANONICAL_TEST_SOCKET="$p"
cd "$root/packages"
if [ "$#" = 1 ] && [ -x "$1" ]; then
   # A pre-built test binary is useful on offline or resource-limited machines.
   "$1" committed_canonical_request_is_the_reviewed_request --ignored --nocapture
elif [ "$#" = 0 ]; then
   cargo test --locked -p windmill --features operational-signing-demo-fixture --lib \
     committed_canonical_request_is_the_reviewed_request -- --ignored --nocapture
else
   echo 'usage: test-canonical-reload.sh [prebuilt-test-binary]' >&2; exit 2
fi

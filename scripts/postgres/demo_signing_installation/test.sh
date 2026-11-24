#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
# A new, private socket-only cluster; deliberately never uses a supplied DB/DSN.
set -euo pipefail
umask 077
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
unset PGHOST PGHOSTADDR PGPORT PGDATABASE PGUSER PGPASSWORD PGSERVICE PGSERVICEFILE PGOPTIONS
export LC_ALL=C
tmp=$(mktemp -d "${TMPDIR:-/tmp}/demo-signing-pg.XXXXXXXX")
export PGHOST="$tmp" PGPORT=35431 PGUSER=fixture_owner
bin=${PG_BINDIR:-$(pg_config --bindir)}
"$bin/initdb" -D "$tmp/db" -U "$PGUSER" -A reject --auth-local=trust --no-locale >"$tmp/init.log"
"$bin/pg_ctl" -D "$tmp/db" -l "$tmp/server.log" -o "-k '$tmp' -h '' -p $PGPORT -c max_connections=15" -w start >/dev/null
trap '"$bin/pg_ctl" -D "$tmp/db" -m fast -w stop >/dev/null; rm -rf -- "$tmp"' EXIT
psql() { "$bin/psql" -X -q -v ON_ERROR_STOP=1 "$@"; }
psql -d postgres -c "CREATE DATABASE signing_test_template"
export PGDATABASE=signing_test_template
psql -c 'CREATE EXTENSION pgcrypto; CREATE EXTENSION unaccent;'
# Apply the actual application schema, one transaction per migration; stop at this feature.
for directory in "$root"/hasura/migrations/backend-db/*; do
    [ "$(basename "$directory" | cut -d_ -f1)" -le 1794700002000 ] || continue
    { echo 'BEGIN;'; cat "$directory/up.sql"; echo; echo 'COMMIT;'; } >>"$tmp/migrations.sql"
done
PGOPTIONS='-c client_min_messages=warning' psql -f "$tmp/migrations.sql" >"$tmp/migrations.log"
# Actual fixture worker login: never test its definer access as the installer.
psql -c 'CREATE ROLE os14990_windmill LOGIN; GRANT sequent_os_approve TO os14990_windmill;'
psql -d postgres -c "CREATE DATABASE operational_demo_14990 TEMPLATE signing_test_template"
psql -d postgres -c "CREATE DATABASE ordinary_hosted TEMPLATE signing_test_template"
for db in ordinary_hosted operational_demo_14990; do
    export PGDATABASE="$db"
    psql -c "INSERT INTO sequent_backend.tenant(id,slug) VALUES ('10000000-0000-4000-8000-000000000001','test-first'),('20000000-0000-4000-8000-000000000001','other-existing'); INSERT INTO sequent_backend.election_event (id,tenant_id,encryption_protocol) VALUES ('10000000-0000-4000-8000-000000000002','10000000-0000-4000-8000-000000000001','RSA256');"
    if [ "$db" = ordinary_hosted ]; then
        if psql -f "$here/enable.sql" -v tenant=10000000-0000-4000-8000-000000000001 -v event=10000000-0000-4000-8000-000000000002 -v run_id=10000000-0000-4000-8000-000000000003 -v backend=softhsm-test >"$tmp/refusal" 2>&1; then
            echo "hosted accepted demo activation" >&2; exit 1
        fi
        grep -F 'dedicated demo database only' "$tmp/refusal" >/dev/null
    else
        for _ in 1 2; do psql -f "$here/enable.sql" -v tenant=10000000-0000-4000-8000-000000000001 -v event=10000000-0000-4000-8000-000000000002 -v run_id=10000000-0000-4000-8000-000000000003 -v backend=softhsm-test; done
    fi
    psql -f "$here/assertions.sql"
    PGUSER=os14990_windmill psql -f "$here/worker-profile.sql"
    echo "PASS $db: installation, new-tenant default and profile admission"
done
# Changing the provider/run or even removing the scope of a previously installed device refuses.
if psql -f "$here/enable.sql" -v tenant=10000000-0000-4000-8000-000000000001 -v event=10000000-0000-4000-8000-000000000002 -v run_id=10000000-0000-4000-8000-000000000003 -v backend=yubihsm >"$tmp/switch" 2>&1; then exit 1; fi
grep -F 'installation identity cannot change' "$tmp/switch" >/dev/null
psql -d postgres -c "ALTER DATABASE operational_demo_14990 RENAME TO restored_without_its_installation"
PGDATABASE=restored_without_its_installation psql -f "$here/restore.sql"
echo 'PASS installed provider switch and renamed restore both refused'

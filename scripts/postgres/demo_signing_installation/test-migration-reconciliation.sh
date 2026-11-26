#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
# No external DSN; every case runs on the fresh private socket-only cluster.
set -euo pipefail
umask 077
root=$(cd "$(dirname "$0")/../../.." && pwd)
m="$root/hasura/migrations/backend-db"
cast="$m/1794700001600_cast_vote_seal_key"
fix="$m/1794700001610_operational_signing_demo_installation"
pg=${PG_BINDIR:-$(pg_config --bindir)}
unset PGHOSTADDR PGPASSWORD PGPASSFILE PGSERVICE PGSERVICEFILE PGOPTIONS
dir=$(mktemp -d /tmp/hsm-migration.XXXXXXXX)
export PGHOST="$dir" PGPORT=35431 PGUSER=fixture_owner PGDATABASE=postgres LC_ALL=C
cleanup() {
    "$pg/pg_ctl" -D "$dir/data" -m immediate -w stop >/dev/null 2>&1 || :
    rm -rf -- "$dir"
}
trap cleanup EXIT
"$pg/initdb" -D "$dir/data" -U "$PGUSER" -A reject --auth-local=trust --no-locale -E UTF8 >"$dir/init.log"
"$pg/pg_ctl" -D "$dir/data" -l "$dir/server.log" -o "-k '$dir' -h '' -p $PGPORT" -w start >/dev/null
psql() { "$pg/psql" -X -qAt -v ON_ERROR_STOP=1 "$@"; }
sql() { psql -c "$1"; }
apply() { psql -1 -f "$1" >"$dir/last.log"; }
fail() { echo "$*" >&2; exit 1; }
assert_sql() { [ "$(sql "$1")" = t ] || fail "failed: $1"; }
refuses() {
    local want=$1
    shift
    if "$@" >"$dir/refusal" 2>&1; then fail "expected refusal: $want"; fi
    grep -F "$want" "$dir/refusal" >/dev/null || { cat "$dir/refusal" >&2; fail "expected: $want"; }
}
# Hasura uses numeric identities. A plain sorted psql loop used to apply
# both 1600 directories and conceal the duplicate rejected by the real CLI.
for path in "$m"/*/; do basename "$path"; done | awk -F_ '
    !/^[0-9]+_/ || seen[sprintf("%.0f", $1+0)]++ { print "Invalid or duplicate migration: " $0 > "/dev/stderr"; bad=1 }
    END {exit bad}'
extract() { awk -v t="$1" '$0=="    " t "_sql constant text := $" t "_sql$" {inside=1;next}
    $0=="$" t "_sql$;" {inside=0} inside' "$fix/up.sql" >"$2"; }
extract cast "$dir/old-cast.sql"
extract demo "$dir/old-demo.sql"
cmp "$cast/up.sql" "$dir/old-cast.sql"
digest() {
    if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | awk '{print $1}'
}
[ "$(digest "$dir/old-cast.sql")" = 9ba93b6f5759afc017cffa249a8604361be9fa7184a5202484ee1392f3fdc450 ] || fail 'canonical 1600 changed'
[ "$(digest "$dir/old-demo.sql")" = 4ffb5b3a6520e1d324ede2ce7d0706f84619439a118a2df7cd3e67cd4685a8c4 ] || fail 'historical demo DDL changed'

sql 'CREATE DATABASE before_1600'
export PGDATABASE=before_1600
sql 'CREATE EXTENSION pgcrypto; CREATE EXTENSION unaccent;'
: >"$dir/before.sql"; : >"$dir/later.sql"; : >"$dir/legacy-hsm.sql"; : >"$dir/pending-extra.sql"
append_migration() { { echo 'BEGIN;'; cat "$1/up.sql"; echo; echo 'COMMIT;'; } >>"$2"; }
for d in "$m"/*/; do
    n=$(basename "$d"); n=${n%%_*}
    if [ "$n" -lt 1794700001600 ]; then
        append_migration "$d" "$dir/before.sql"
    elif [ "$n" -gt 1794700001610 ]; then
        append_migration "$d" "$dir/later.sql"
        # Only this portable-independent HSM lineage could have been
        # applied alongside the old demo 1600. Portable 1650 guards
        # canonical cast sealing and must remain pending on this upgrade.
        if [ "$n" -ge 1794700001700 ] && [ "$n" -le 1794700002340 ]; then
            append_migration "$d" "$dir/legacy-hsm.sql"
        else
            append_migration "$d" "$dir/pending-extra.sql"
        fi
    fi
done
export PGOPTIONS='-c client_min_messages=error'
psql -f "$dir/before.sql" >"$dir/before.log"
export PGDATABASE=postgres
# Always use the real, dedicated database name when seeding an opted-in
# demo: its original CHECK and registration routine prohibit a lookalike.
db=operational_demo_14990
new_case() { export PGDATABASE=postgres; sql "CREATE DATABASE $db TEMPLATE before_1600"; export PGDATABASE=$db; }
finish_case() { export PGDATABASE=postgres; sql "DROP DATABASE $db"; }
t=10000000-0000-4000-8000-000000000001
e=10000000-0000-4000-8000-000000000002
r=10000000-0000-4000-8000-000000000003
other=20000000-0000-4000-8000-000000000001
key=046b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c2964fe342e2fe1a7f9b8ee7eb4a7c0f9e162bce33576b315ececbb6406837bf51f5
seed_base() { sql "INSERT INTO sequent_backend.tenant(id,slug) VALUES ('$t','fixture'),('$other','unrelated');
    INSERT INTO sequent_backend.election_event(id,tenant_id,encryption_protocol) VALUES ('$e','$t','RSA256');
    INSERT INTO sequent_backend.election(id,tenant_id,election_event_id) VALUES ('10000000-0000-4000-8000-000000000004','$t','$e');"; }
publish() { assert_sql "SET ROLE sequent_os_cast_vote_key_publisher;
    SELECT sequent_backend.cast_vote_seal_publish('$t','$e','cast-vote-seal/$t/$e',1,decode('$key','hex'));"; }
register() { sql "SELECT sequent_backend.operational_register_demo_scope('$t','$e','$r','softhsm-test')" >/dev/null; }
snapshot() {
    local hc=$1 hd=$2 tab
    # Preserve complete rows, including timestamps and the immutable key.
    for tab in tenant election_event election operational_signing_required_tenant operational_setup_installation; do
        sql "SELECT row_to_json(x) FROM sequent_backend.$tab x ORDER BY row_to_json(x)::text"
    done
    for tab in cast_vote_sealing_required cast_vote_seal_key operational_signing_demo_install operational_signing_demo_scope; do
        case "$tab" in cast*) [ "$hc" = t ] || continue;; *) [ "$hd" = t ] || continue;; esac
        sql "SELECT row_to_json(x) FROM sequent_backend.$tab x ORDER BY row_to_json(x)::text"
    done
    # The pre-existing objects, owners, grants, bodies and trigger OIDs must
    # not be replaced or re-granted, including the operator's claim revocation.
    local pred="(('$hc'::boolean AND name ~ '^cast_vote_seal') OR ('$hd'::boolean AND name ~ '^(operational_signing_demo_|operational_demo_|operational_register_demo_scope$)'))"
    sql "WITH objects(name,identity) AS (
        SELECT relname,concat_ws('|',oid,relowner,relacl) FROM pg_class WHERE relnamespace='sequent_backend'::regnamespace
        UNION ALL SELECT proname,concat_ws('|',oid,proowner,proacl,prosrc) FROM pg_proc WHERE pronamespace='sequent_backend'::regnamespace
        UNION ALL SELECT t.tgname,concat_ws('|',t.oid,t.tgenabled,pg_get_triggerdef(t.oid)) FROM pg_trigger t
            JOIN pg_class c ON c.oid=t.tgrelid WHERE c.relnamespace='sequent_backend'::regnamespace AND NOT t.tgisinternal)
        SELECT identity FROM objects WHERE $pred OR name='operational_setup_claim_generation' ORDER BY name,identity;"
}

new_case
apply "$cast/up.sql"; apply "$fix/up.sql"; psql -f "$dir/later.sql" >"$dir/later.log"
assert_sql 'SELECT NOT EXISTS (TABLE sequent_backend.tenant) AND NOT EXISTS (TABLE sequent_backend.election_event)
    AND NOT EXISTS (TABLE sequent_backend.operational_signing_demo_install) AND NOT EXISTS (TABLE sequent_backend.cast_vote_seal_key)'
finish_case
echo 'PASS clean: all ordered backend migrations apply, no bootstrapped tenants/events or policy rows'

for state in cast_only demo_only both; do
    new_case; seed_base; hc=f; hd=f
    if [ "$state" != demo_only ]; then apply "$dir/old-cast.sql"; publish; hc=t; fi
    if [ "$state" != cast_only ]; then apply "$dir/old-demo.sql"; hd=t; fi
    # Simulate an existing database whose Hasura catalogue already has 1600
    # and the later HSM versions. Apply pending integration extras (such as
    # portable 1650) after reconciliation, not as if they were in that lineage.
    psql -f "$dir/legacy-hsm.sql" >"$dir/legacy-hsm.log"
    if [ "$hd" = t ]; then register; fi
    snapshot "$hc" "$hd" >"$dir/before"
    apply "$fix/up.sql"; apply "$fix/up.sql"
    psql -f "$dir/pending-extra.sql" >"$dir/pending-extra.log"
    snapshot "$hc" "$hd" >"$dir/after"; diff -u "$dir/before" "$dir/after"
    assert_sql "SELECT NOT sequent_backend.operational_signing_hsm_required('$other') AND (SELECT count(*) FROM sequent_backend.tenant)=2
        AND (SELECT count(*) FROM sequent_backend.election_event)=1 AND (SELECT count(*) FROM sequent_backend.election)=1
        AND NOT has_function_privilege('sequent_os_approve','sequent_backend.cast_vote_seal_publish(uuid,uuid,text,integer,bytea)','EXECUTE')
        AND NOT has_table_privilege('sequent_os_cast_vote_key_publisher','sequent_backend.cast_vote_seal_key','INSERT')
        AND NOT has_function_privilege('sequent_os_approve','sequent_backend.operational_register_demo_scope(uuid,uuid,uuid,text)','EXECUTE')"
    publish
    assert_sql "SELECT required AND fingerprint=encode(sha256(decode('$key','hex')),'hex') FROM sequent_backend.cast_vote_seal_state('$t','$e')"
    refuses 'insert-only' sql 'UPDATE sequent_backend.cast_vote_seal_key SET key_version=2;'
    if [ "$hd" = t ]; then
        assert_sql "SET ROLE sequent_os_approve; SELECT sequent_backend.operational_signing_demo_fixture_id('$t','$e','initialize-voting','softhsm-test')='$r'::uuid"
        assert_sql "SELECT NOT has_function_privilege('sequent_os_setup_device','sequent_backend.operational_setup_claim_generation(uuid,uuid,uuid,uuid)','EXECUTE')"
        refuses 'operational demo is installed' apply "$fix/down.sql"
        refuses 'immutable' sql 'TRUNCATE sequent_backend.operational_signing_demo_scope'
    else
        # Empty demo can be reversed, but the canonical key is never removed.
        apply "$fix/down.sql"
        assert_sql "SELECT required FROM sequent_backend.cast_vote_seal_state('$t','$e')"
        apply "$fix/up.sql"
    fi
    finish_case; echo "PASS $state: rows, identities, permissions, enabled guards and safe rollback"
done

# Broken or unrecognized 1600 must fail atomically. Even if the other half
# was wholly absent, no part of its DDL survives a failed reconciliation.
new_case
refuses 'neither cast sealing nor operational demo' apply "$fix/up.sql"
apply "$dir/old-demo.sql"
sql 'ALTER TABLE sequent_backend.operational_signing_demo_scope DISABLE TRIGGER operational_demo_scope_immutable'
refuses 'Incomplete or changed 1600 trigger' apply "$fix/up.sql"
assert_sql "SELECT to_regclass('sequent_backend.cast_vote_seal_key') IS NULL"
sql 'ALTER TABLE sequent_backend.operational_signing_demo_scope ENABLE TRIGGER operational_demo_scope_immutable;
    GRANT EXECUTE ON FUNCTION sequent_backend.operational_register_demo_scope(uuid,uuid,uuid,text) TO PUBLIC'
refuses 'Unexpected EXECUTE grants' apply "$fix/up.sql"
assert_sql "SELECT to_regclass('sequent_backend.cast_vote_seal_key') IS NULL AND has_function_privilege(
    'sequent_os_approve','sequent_backend.operational_register_demo_scope(uuid,uuid,uuid,text)','EXECUTE')"
finish_case
new_case; apply "$dir/old-cast.sql"
sql 'ALTER TABLE sequent_backend.cast_vote_seal_key DROP CONSTRAINT cast_vote_seal_key_key_version_check'
refuses 'Unexpected or inactive constraints' apply "$fix/up.sql"
assert_sql "SELECT to_regclass('sequent_backend.operational_signing_demo_install') IS NULL"
finish_case
new_case
sql "CREATE FUNCTION sequent_backend.cast_vote_seal_state(uuid,uuid) RETURNS boolean LANGUAGE sql AS 'SELECT false'"
refuses 'Incomplete or unexpected 1600 table' apply "$fix/up.sql"
assert_sql "SELECT to_regclass('sequent_backend.operational_signing_demo_install') IS NULL"
finish_case
echo 'PASS unknown and partial catalogs, disabled immutability trigger, unauthorized grant, removed check and orphan routine'

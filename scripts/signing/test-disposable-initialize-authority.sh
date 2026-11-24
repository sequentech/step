#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
# Focused fixture authority + PostgreSQL role/CSR test. The two synthetic
# CSRs here are generated using OpenSSL; this is not a PKCS#11/browser test.
set -eu
umask 077
unset PGHOST PGHOSTADDR PGDATABASE PGUSER PGPORT PGPASSWORD PGSERVICE PGSERVICEFILE PGOPTIONS PGAPPNAME
export LC_ALL=C
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
export PATH="${PG_BINDIR:-$(pg_config --bindir)}:$PATH"
p=$(mktemp -d "${TMPDIR:-/tmp}/demo-issuer.XXXXXXXX")
p=$(cd "$p" && pwd -P)
export PGHOST="$p" PGPORT=46333 PGDATABASE=postgres
initdb -D "$p/pg" -U "$(id -un)" -A reject --auth-local=trust --no-locale >"$p/init.log"
pg_ctl -D "$p/pg" -l "$p/pg.log" -o "-k '$p' -h '' -p $PGPORT" -w start >/dev/null
issuer_pid=
cleanup() {
    if [ -n "$issuer_pid" ]; then kill "$issuer_pid" 2>/dev/null || true; wait "$issuer_pid" 2>/dev/null || true; fi
    pg_ctl -D "$p/pg" -m fast -w stop >/dev/null
    rm -rf -- "$p"
}
trap cleanup EXIT
sql() { psql -X -q -A -t -v ON_ERROR_STOP=1 "$@"; }
sql -c 'CREATE DATABASE operational_demo_14990'
export PGDATABASE=operational_demo_14990
{ echo 'CREATE EXTENSION pgcrypto; CREATE EXTENSION unaccent;'
  for d in "$root"/hasura/migrations/backend-db/*; do
    v=${d##*/}; [ "${v%%_*}" -le 1794700001700 ] || continue
    echo 'BEGIN;'; cat "$d/up.sql"; echo; echo 'COMMIT;'
  done
} >"$p/migrations.sql"
PGOPTIONS='-c client_min_messages=warning' sql -f "$p/migrations.sql" >"$p/migration.log"
tenant=a0000000-0000-4000-8000-000000000001
event=a0000000-0000-4000-8000-000000000002
post=a0000000-0000-4000-8000-000000000003
request=a0000000-0000-4000-8000-000000000004
run=a0000000-0000-4000-8000-000000000005
officer=a0000000-0000-4000-8000-0000000000aa
first=a0000000-0000-4000-8000-0000000000a1
second=a0000000-0000-4000-8000-0000000000a2
ids=(-v "tenant=$tenant" -v "event=$event" -v "post=$post" -v "request=$request" -v "run_id=$run" -v 'backend=softhsm-test')
sql "${ids[@]}" <<'SQL' >/dev/null
CREATE ROLE sample_harvest LOGIN; GRANT sequent_os_approve TO sample_harvest;
CREATE ROLE sample_generation LOGIN; GRANT sequent_os_setup_device TO sample_generation;
INSERT INTO sequent_backend.tenant (id,slug) VALUES (:'tenant','fixture-only');
INSERT INTO sequent_backend.election_event (id,tenant_id,encryption_protocol) VALUES (:'event',:'tenant','RSA256');
INSERT INTO sequent_backend.election (id,tenant_id,election_event_id) VALUES (:'post',:'tenant',:'event');
SQL
sql "${ids[@]}" -f "$root/scripts/postgres/demo_signing_installation/enable.sql" >/dev/null
payload='{"purpose":"fixture-authority-test"}'
hash=$(printf %s "$payload" | openssl dgst -sha256 | awk '{print $NF}')
sql "${ids[@]}" -v "hash=$hash" -v "payload=$payload" <<'SQL'
INSERT INTO sequent_backend.signing_request
    (id,tenant_id,election_event_id,action,election_id,scope_key,subject,canonical_payload,payload_sha256,
     code,rule_revision,rule_snapshot,required,requested_by,requested_by_username,expires_at)
VALUES (:'request',:'tenant',:'event','initialize-voting',:'post','test','{}',:'payload',:'hash',
    'CA-TEST',1,'{}',2,'requester','synthetic',clock_timestamp()+interval '15 minutes');
SQL
mkdir -m 700 "$p/ca"
authority=(python3 -B "$here/disposable_initialize_authority.py" --state "$p/ca" --socket "$p" --port "$PGPORT" --tenant "$tenant" --event "$event" --run "$run" --backend softhsm-test)
"${authority[@]}" create-ca
"${authority[@]}" preauthorize "$officer" "$first" "$second"
"${authority[@]}" serve-preauthorized-pair "$officer" "$first" "$second" --timeout 60 >"$p/issuer.log" 2>&1 &
issuer_pid=$!
# Generation enters only through the real demo claim wrapper, with the generic
# entry revoked. The fixture CA receives the public part only, not the keys.
for who in "$first" "$second"; do
    attempt=$(sql -c 'SELECT gen_random_uuid()')
    common=("${ids[@]}" -v "official=$who" -v "attempt=$attempt" -v "hash=$hash")
    started=$(sql -U sample_harvest "${common[@]}" <<'SQL'
SELECT sequent_backend.operational_initialize_start_self_setup(
    :'tenant',:'event',:'request',:'post',:'official',:'hash',clock_timestamp(),:'attempt');
SQL
)
    [ "$started" = t ] || { echo 'valid first use refused' >&2; exit 1; }
    fence=$(sql -U sample_generation "${common[@]}" <<'SQL'
SELECT fence FROM sequent_backend.operational_initialize_claim_self_setup(:'tenant',:'event',:'official',:'attempt');
SQL
)
    [ -n "$fence" ] || { echo 'scoped claim refused' >&2; exit 1; }
    openssl req -new -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -keyout "$p/$who.test-key" \
      -out "$p/$who.csr" -subj "/CN=$who/OU=$tenant" >/dev/null 2>&1
    spki=$(openssl req -in "$p/$who.csr" -pubkey -noout | openssl pkey -pubin -outform DER | openssl dgst -sha256 | awk '{print $NF}')
    enrollment=$(sql -U sample_generation "${common[@]}" -v "fence=$fence" -v "spki=$spki" -v "csr=$(cat "$p/$who.csr")" <<'SQL'
SELECT sequent_backend.operational_setup_complete_generation(:'tenant',:'event',:'official',:'attempt',:'fence',
    :'spki',encode(sha256(convert_to('synthetic test attestation','UTF8')),'hex'),:'csr');
SQL
)
    [ -n "$enrollment" ] || { echo 'enrollment evidence refused' >&2; exit 1; }
done
wait "$issuer_pid" || { cat "$p/issuer.log" >&2; exit 1; }
issuer_pid=
result=$(sql "${ids[@]}" <<'SQL'
SELECT (SELECT count(DISTINCT spki_sha256) FROM sequent_backend.staff_certificate
         WHERE tenant_id=:'tenant' AND election_event_id=:'event' AND status='active'),
       (SELECT count(*) FROM sequent_backend.signing_approval WHERE request_id=:'request'),
       (SELECT status FROM sequent_backend.signing_request WHERE id=:'request');
SQL
)
[ "$result" = '2|0|waiting' ] || { echo "unexpected fixture result: $result" >&2; exit 1; }
# The independent read-only observer must agree: no receipt/effect after setup.
sql "${ids[@]}" -v "hash=$hash" -v "first=$first" -v "second=$second" -v "run=$run" -v expected=0 \
  -f "$root/scripts/operational-signing-demo/observe-initialize.sql" >/dev/null
# Bound replay: already-attached IDs aren't issued a second certificate.
"${authority[@]}" serve-preauthorized-pair "$officer" "$first" "$second" --timeout 2
echo 'PASS: two role-scoped claims, two distinct issued certificates, zero approvals, request remains waiting'

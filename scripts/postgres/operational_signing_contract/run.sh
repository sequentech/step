#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# Contract test of the operational signing ledger migrations 1794700001000-1400.
#
# Starts one disposable postgres:18 container (no network, 1 CPU, 1 GiB),
# and inside it:
#   1. applies the whole Step backend-db chain plus these migrations, each
#      file in its own transaction, exactly as the CI migration job does;
#   2. runs the role-by-role assertions of 20_assertions.sql;
#   3. requires every stateful rollback to refuse while its ledger has rows;
#   4. on a second database: snapshots owners and privileges, applies, rolls
#      back and reapplies only these migrations and requires the rollback to
#      restore every owner, security mode and privilege;
#
# The whole chain's apply, roll back and reapply is checked separately by
# scripts/postgres/check_migrations.sh.
#
# Usage: scripts/postgres/operational_signing_contract/run.sh [backend-db-dir]
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
migrations=$(cd "${1:-$here/../../../hasura/migrations/backend-db}" && pwd)
image=${POSTGRES_IMAGE:-postgres:18}
name="os-contract-test-$$-$(date +%s)"

cleanup() { docker rm --force --volumes "$name" >/dev/null 2>&1 || true; }
trap cleanup EXIT

docker run --detach --name "$name" --network none --cpus 1 --memory 1g \
    --env POSTGRES_PASSWORD=unused-local-socket-only \
    --volume "$migrations:/migrations:ro" \
    --volume "$here:/contract-test:ro" \
    --volume "$here/../../../packages/windmill/src/services/signing/sql/operational_quorum.sql:/windmill-quorum.sql:ro" "$image" >/dev/null

# The entrypoint restarts the server once after initialisation.
for _ in $(seq 1 120); do
    if docker logs "$name" 2>&1 | grep -q 'PostgreSQL init process complete' \
        && docker exec "$name" pg_isready --quiet --username postgres; then
        ready=1
        break
    fi
    sleep 0.5
done
[ "${ready:-0}" = 1 ] || { echo "PostgreSQL did not start" >&2; docker logs "$name" >&2; exit 1; }

# As root: migration files may be readable by their owner only.
docker exec --interactive "$name" bash -s <<'INNER'
set -euo pipefail
export PGUSER=postgres PGOPTIONS='-c client_min_messages=warning'
run() { psql --no-psqlrc --quiet --set=ON_ERROR_STOP=1 "$@" >/dev/null; }
versions() { find -L "$1" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort -t_ -k1,1n; }
apply() { # database direction directory versions...
    local database=$1 direction=$2 directory=$3 version
    shift 3
    for version in "$@"; do
        run --dbname="$database" --single-transaction --file="$directory/$version/$direction.sql"
    done
}
# The contract under test: 1794700001000 to 1794700001400. Everything before
# it is the chain it builds on; anything after it is not applied.
mkdir -p /step-migrations /new-migrations
for path in /migrations/*/; do
    version=$(basename "$path")
    number=${version%%_*}
    if [ "$number" -lt 1794700001000 ]; then
        ln -s "$path" "/step-migrations/$version"
    elif [ "$number" -le 1794700001400 ]; then
        ln -s "$path" "/new-migrations/$version"
    fi
done
mapfile -t step_versions < <(versions /step-migrations)
mapfile -t new_versions < <(versions /new-migrations)
[ "${#new_versions[@]}" = 5 ] || { echo "expected 5 contract migrations, found ${#new_versions[@]}" >&2; exit 1; }
reversed=()
for ((index = ${#new_versions[@]} - 1; index >= 0; index--)); do
    reversed+=("${new_versions[index]}")
done
fresh() {
    run --command="CREATE DATABASE $1"
    run --dbname="$1" --command='CREATE EXTENSION IF NOT EXISTS pgcrypto' \
        --command='CREATE EXTENSION IF NOT EXISTS unaccent'
    apply "$1" up /step-migrations "${step_versions[@]}"
}

echo "== 1. apply ${#step_versions[@]} Step migrations and ${#new_versions[@]} new ones"
fresh contract
apply contract up /new-migrations "${new_versions[@]}"

echo "== 2. role-by-role assertions"
run --dbname=contract --file=/contract-test/10_fixture.sql
PGOPTIONS='' psql --no-psqlrc --dbname=contract --set=ON_ERROR_STOP=1 --output=/dev/null \
    --set=quorum_sql="$(cat /windmill-quorum.sql)" --file=/contract-test/20_assertions.sql >/tmp/assertions.log 2>&1 || {
    cat /tmp/assertions.log
    echo "ASSERTIONS FAILED"
    exit 1
}
grep -q '^ASSERTIONS-COMPLETE$' /tmp/assertions.log || grep -q 'ASSERTIONS-COMPLETE' /tmp/assertions.log \
    || { cat /tmp/assertions.log; echo "ASSERTIONS INCOMPLETE"; exit 1; }
if grep -v -e 'NOTICE:  ok - ' -e 'ASSERTIONS-COMPLETE' /tmp/assertions.log | grep -q .; then
    grep -v -e 'NOTICE:  ok - ' -e 'ASSERTIONS-COMPLETE' /tmp/assertions.log
    echo "UNEXPECTED OUTPUT"
    exit 1
fi
sed -n 's/^.*NOTICE:  \(ok - .*\)$/\1/p' /tmp/assertions.log
echo "assertions passed: $(grep -c 'NOTICE:  ok - ' /tmp/assertions.log)"

echo "== 3. stateful rollbacks refuse while the ledger has rows"
for version in "${reversed[@]}"; do
    case $version in
    *_operational_initialize_broker | *_operational_setup_generation | *_operational_miru_registry)
        if psql --no-psqlrc --quiet --set=ON_ERROR_STOP=1 --dbname=contract --single-transaction \
            --file="/new-migrations/$version/down.sql" >/tmp/down.log 2>&1; then
            echo "rollback of $version did not refuse" >&2
            exit 1
        fi
        grep -q 'refuse rollback' /tmp/down.log || { cat /tmp/down.log >&2; exit 1; }
        echo "ok - $version refuses: $(grep -o 'operational .* refuse rollback' /tmp/down.log)"
        ;;
    esac
done

echo "== 4. rollback restores every owner, security mode and privilege"
fresh cycle
snapshot() { psql --no-psqlrc --quiet --dbname=cycle --file=/contract-test/30_privileges_snapshot.sql >"$1"; }
snapshot /tmp/before.txt
apply cycle up /new-migrations "${new_versions[@]}"
snapshot /tmp/applied.txt
apply cycle down /new-migrations "${reversed[@]}"
snapshot /tmp/rolled-back.txt
diff -u /tmp/before.txt /tmp/rolled-back.txt
apply cycle up /new-migrations "${new_versions[@]}"
snapshot /tmp/reapplied.txt
diff -u /tmp/applied.txt /tmp/reapplied.txt
if diff -q /tmp/before.txt /tmp/applied.txt >/dev/null; then
    echo "the snapshot does not see the migrations" >&2
    exit 1
fi
echo "ok - $(wc -l </tmp/before.txt) objects before, $(wc -l </tmp/applied.txt) applied; rollback and reapply are exact"
echo "ok - group roles after rollback: $(psql --no-psqlrc --quiet --tuples-only --no-align --dbname=cycle \
    --command="SELECT count(*) FROM pg_roles WHERE rolname LIKE 'sequent\\_os\\_%' AND NOT rolcanlogin") (left in place)"

INNER
echo "ALL PASSED"

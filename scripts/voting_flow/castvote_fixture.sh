#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# Start a PostgreSQL 18 container with the fixture schema that database.py
# builds for the voting-flow regressions, and print its database URL. Run the
# ignored Windmill database tests against it from packages/:
#
#   export CAST_VOTE_TEST_DATABASE_URL=$(../scripts/voting_flow/castvote_fixture.sh)
#   cargo test --locked -p windmill --lib services::insert_cast_vote::tests -- --ignored
#   docker rm --force --volumes step-castvote-fixture
#
# Usage: scripts/voting_flow/castvote_fixture.sh [container-name] [host-port]
# An existing container with the same name is left untouched. Without a port,
# Docker publishes a free one on 127.0.0.1. Failed fixture setup removes the
# container; after success, the caller owns its cleanup.

set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
migrations=$root/hasura/migrations/backend-db
name=${1:-step-castvote-fixture}
port=${2:-}

docker create --name "$name" --publish "127.0.0.1:$port:5432" \
    --env POSTGRES_HOST_AUTH_METHOD=trust \
    postgres:18-bookworm -c shared_preload_libraries=pg_stat_statements >/dev/null
cleanup_on_error() {
    if [ "$?" -ne 0 ]; then
        docker rm --force --volumes "$name" >/dev/null 2>&1 || true
    fi
}
trap cleanup_on_error EXIT
docker start "$name" >/dev/null

# The image only listens on TCP once its initialization has finished.
ready=false
for _ in $(seq 60); do
    if docker exec "$name" pg_isready --host=127.0.0.1 --quiet; then
        ready=true
        break
    fi
    sleep 1
done
if [ "$ready" != true ]; then
    docker logs "$name" 2>&1 | tail -n 20 >&2
    echo "PostgreSQL did not start" >&2
    exit 1
fi

fixture() {
    docker exec --interactive "$name" \
        psql --username=postgres --no-psqlrc --quiet --set=ON_ERROR_STOP=1 "$@" >/dev/null
}
fixture <"$root/scripts/voting_flow/schema.sql"
fixture --single-transaction <"$migrations/1788765000000_serialize_cast_vote_area_checks/up.sql"
fixture --command='CREATE TRIGGER check_revote_limit_trigger
    BEFORE INSERT ON sequent_backend.cast_vote
    FOR EACH ROW EXECUTE FUNCTION check_revote_limit()'
fixture --single-transaction <"$migrations/1788765000002_materialize_voting_windows/up.sql"
fixture --single-transaction <"$migrations/1789420000001_voting_windows_follow_channels/up.sql"

echo "postgres://postgres@$(docker port "$name" 5432/tcp | head -n 1)/postgres"

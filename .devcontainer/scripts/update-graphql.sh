#!/bin/bash -i
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -e -o pipefail

source .devcontainer/.env
# devenv prepends its Nix OpenSSL to LD_LIBRARY_PATH, but the devcontainer's
# system Docker CLI must load the Ubuntu-compatible OpenSSL libraries.
env -u LD_LIBRARY_PATH docker compose restart graphql-engine

# Generate graphql schema.
#
# graphql-engine defines no healthcheck, so readiness is defined as "the
# introspection we need succeeds" and retried until a deadline.
cd packages/admin-portal

# Written to a temporary file in the destination directory and renamed on
# success, so a failed run leaves the tracked schema untouched rather than
# truncated.
SCHEMA_TMP="$(mktemp ./.graphql.schema.json.XXXXXX)"
GQ_STDERR="$(mktemp)"
trap 'rm -f "${SCHEMA_TMP}" "${GQ_STDERR}"' EXIT INT TERM

# Compose feeds graphql-engine HASURA_GRAPHQL_ADMIN_SECRET from
# KEYCLOAK_ADMIN_CLIENT_SECRET locally, but remote deployments set it
# independently.
HASURA_ADMIN_SECRET="${HASURA_GRAPHQL_ADMIN_SECRET:-${KEYCLOAK_ADMIN_CLIENT_SECRET:?neither HASURA_GRAPHQL_ADMIN_SECRET nor KEYCLOAK_ADMIN_CLIENT_SECRET is set; expected one from .devcontainer/.env}}"

HASURA_READY_TIMEOUT_SECS="${HASURA_READY_TIMEOUT_SECS:-120}"
deadline=$((SECONDS + HASURA_READY_TIMEOUT_SECS))

# graphql-engine restarts always, so it reads as not running for a moment on
# every restart. Only consecutive observations mean it is really down.
not_running_count=0

while true; do
    remaining=$((deadline - SECONDS))
    if [ "${remaining}" -le 0 ]; then
        echo "graphql-engine was not ready within ${HASURA_READY_TIMEOUT_SECS}s; last introspection error:" >&2
        cat "${GQ_STDERR}" >&2
        echo "check 'docker logs hasura'" >&2
        exit 1
    fi

    # Each attempt is bounded by the time left on the overall deadline, so a
    # request that connects and then stalls cannot hang the script past it.
    if timeout "${remaining}s" gq http://graphql-engine:8080/v1/graphql \
            -H "X-Hasura-Admin-Secret: ${HASURA_ADMIN_SECRET}" \
            --introspect \
            --format json \
            > "${SCHEMA_TMP}" 2>"${GQ_STDERR}" \
        && [ -s "${SCHEMA_TMP}" ]; then
        break
    fi

    # A container that stays down will never become ready, so fail now with a
    # pointer to the cause rather than waiting out the whole timeout. A failed
    # migration leaves graphql-engine in a restart loop and lands here.
    if [ "$(env -u LD_LIBRARY_PATH docker inspect -f '{{.State.Running}}' hasura 2>/dev/null)" != "true" ]; then
        not_running_count=$((not_running_count + 1))
        if [ "${not_running_count}" -ge 3 ]; then
            echo "graphql-engine is not running; check 'docker logs hasura'" >&2
            exit 1
        fi
    else
        not_running_count=0
    fi

    sleep 2
done

mv "${SCHEMA_TMP}" graphql.schema.json
rm -f "${GQ_STDERR}"
trap - EXIT INT TERM

# Copy the schema to the apps
cd ..
cp admin-portal/graphql.schema.json voting-portal/graphql.schema.json
cp admin-portal/graphql.schema.json ballot-verifier/graphql.schema.json
cp admin-portal/graphql.schema.json results-portal/graphql.schema.json
cp admin-portal/graphql.schema.json step-cli/src/graphql/schema.json
cp admin-portal/graphql.schema.json .

yarn

# Generate Ts types, functions and graphql queries
yarn generate:admin-portal
yarn generate:voting-portal
yarn generate:ballot-verifier
yarn generate:results-portal

# Format the generated source files
yarn lint:fix && yarn prettify:fix

# Format the generated hasura files
cd ../hasura && yarn && yarn prettify:fix

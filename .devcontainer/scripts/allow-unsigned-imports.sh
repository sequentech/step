#!/bin/bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Lets a tenant of the development stack import unsigned election event files,
# which a tenant without a `configuration_signing` setting refuses. The
# setting's guard only takes a signing policy, so the statement runs with
# triggers off, as Hasura's database superuser. A tenant that has a setting
# keeps it.
#
# Usage: .devcontainer/scripts/allow-unsigned-imports.sh [tenant-id]
# The tenant defaults to $SUPER_ADMIN_TENANT_ID, whose row windmill creates
# when the stack first starts; the script waits for it. Run it again after
# resetting the database.

set -eu -o pipefail
set +x

TENANT_ID="${1:-${SUPER_ADMIN_TENANT_ID:?SUPER_ADMIN_TENANT_ID is not set}}"
if ! [[ "${TENANT_ID}" =~ ^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$ ]]; then
    echo "not a tenant id: ${TENANT_ID}" >&2
    exit 2
fi
ENDPOINT="${HASURA_ENDPOINT:-http://graphql-engine:8080/v1/graphql}"
QUERY_URL="${ENDPOINT%/v1/graphql}/v2/query"
# Compose feeds graphql-engine HASURA_GRAPHQL_ADMIN_SECRET from
# KEYCLOAK_ADMIN_CLIENT_SECRET locally.
ADMIN_SECRET="${HASURA_GRAPHQL_ADMIN_SECRET:-${HASURA_ADMIN_SECRET:-${KEYCLOAK_ADMIN_CLIENT_SECRET:?no Hasura admin secret: set HASURA_GRAPHQL_ADMIN_SECRET or KEYCLOAK_ADMIN_CLIENT_SECRET}}}"
TIMEOUT_SECS="${ALLOW_UNSIGNED_IMPORTS_TIMEOUT_SECS:-300}"

# Prints the result of one run_sql call; the admin secret goes in a header
# read from a file descriptor, so it is not in the process list.
run_sql() {
    local response status
    response="$(jq -n --arg sql "$1" '{type: "run_sql", args: {sql: $sql}}' \
        | curl --silent --show-error \
            --header @<(printf 'x-hasura-admin-secret: %s\n' "${ADMIN_SECRET}") \
            --header 'Content-Type: application/json' \
            --data-binary @- \
            --write-out '\n%{http_code}' \
            "${QUERY_URL}")" || return 1
    status="${response##*$'\n'}"
    response="${response%$'\n'*}"
    if [ "${status}" != 200 ]; then
        echo "Hasura answered HTTP ${status}: ${response}" >&2
        return 1
    fi
    printf '%s\n' "${response}"
}

LAST_ERROR="$(mktemp)"
trap 'rm -f "${LAST_ERROR}"' EXIT
deadline=$((SECONDS + TIMEOUT_SECS))
until run_sql "SELECT 1 FROM sequent_backend.tenant WHERE id = '${TENANT_ID}'" 2>"${LAST_ERROR}" \
        | jq -e '.result | length > 1' > /dev/null; do
    if [ "${SECONDS}" -ge "${deadline}" ]; then
        echo "tenant ${TENANT_ID} did not appear within ${TIMEOUT_SECS}s" >&2
        cat "${LAST_ERROR}" >&2
        exit 1
    fi
    sleep 5
done

run_sql "SET LOCAL session_replication_role = replica; UPDATE sequent_backend.tenant SET settings = COALESCE(settings, '{}'::jsonb) || '{\"configuration_signing\": {\"policy\": \"optional\"}}'::jsonb WHERE id = '${TENANT_ID}' AND settings -> 'configuration_signing' IS NULL" > /dev/null
echo "tenant ${TENANT_ID} takes unsigned election event files unless its setting says otherwise"

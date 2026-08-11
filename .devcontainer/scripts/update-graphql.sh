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
# Readiness is defined as "the introspection we need succeeds", rather than a
# fixed wait or the container healthcheck: the healthcheck has no explicit
# interval, so it inherits Docker's 30s default and can report unhealthy for
# half a minute after Hasura is already serving.
#
# The output goes to a temporary file and is moved into place only on success.
# Redirecting straight into graphql.schema.json truncates it before gq runs, so
# any failure used to leave the tracked schema empty.
cd packages/admin-portal

SCHEMA_TMP="$(mktemp)"
trap 'rm -f "${SCHEMA_TMP}"' EXIT

HASURA_READY_TIMEOUT_SECS="${HASURA_READY_TIMEOUT_SECS:-120}"
deadline=$((SECONDS + HASURA_READY_TIMEOUT_SECS))

until gq http://graphql-engine:8080/v1/graphql \
        -H 'X-Hasura-Admin-Secret: admin' \
        --introspect \
        --format json \
        > "${SCHEMA_TMP}" 2>/dev/null && [ -s "${SCHEMA_TMP}" ]; do
    # A container that is not running will never become ready, so fail now with
    # a pointer to the cause rather than waiting out the whole timeout. A failed
    # migration leaves graphql-engine in a restart loop and lands here.
    if [ "$(env -u LD_LIBRARY_PATH docker inspect -f '{{.State.Running}}' hasura 2>/dev/null)" != "true" ]; then
        echo "graphql-engine is not running; check 'docker logs hasura'" >&2
        exit 1
    fi
    if [ "${SECONDS}" -ge "${deadline}" ]; then
        echo "graphql-engine was not ready within ${HASURA_READY_TIMEOUT_SECS}s; check 'docker logs hasura'" >&2
        exit 1
    fi
    sleep 2
done

mv "${SCHEMA_TMP}" graphql.schema.json
trap - EXIT

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

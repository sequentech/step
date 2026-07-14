#!/bin/bash -i
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex -o pipefail

source .devcontainer/.env
docker compose restart graphql-engine

# graphql-engine applies migrations and metadata before it starts serving,
# so poll the health endpoint instead of sleeping a fixed amount of time
timeout 120 bash -c \
    'until curl -fsS http://graphql-engine:8080/healthz >/dev/null 2>&1; do sleep 2; done'

# Generate graphql schema
cd packages/admin-portal
gq http://graphql-engine:8080/v1/graphql \
    -H 'X-Hasura-Admin-Secret: admin' \
    --introspect  \
    --format json \
    > graphql.schema.json

# Copy the schema to the apps
cd ..
cp admin-portal/graphql.schema.json voting-portal/graphql.schema.json
cp admin-portal/graphql.schema.json ballot-verifier/graphql.schema.json
cp admin-portal/graphql.schema.json step-cli/src/graphql/schema.json
cp admin-portal/graphql.schema.json .

yarn

# Generate Ts types, functions and graphql queries
yarn generate:admin-portal
yarn generate:voting-portal
yarn generate:ballot-verifier

# Format the generated source files
yarn lint:fix && yarn prettify:fix

# Format the generated hasura files
cd ../hasura && yarn && yarn prettify:fix

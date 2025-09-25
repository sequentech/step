#!/bin/bash -i
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex -o pipefail

source .devcontainer/.env
docker compose restart graphql-engine

# graphql-engine needs some waiting time before it's up and working
sleep 10

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

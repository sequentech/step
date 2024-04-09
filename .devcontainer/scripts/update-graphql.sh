#!/bin/bash -i

set -ex -o pipefail

source .devcontainer/.env
docker compose restart graphql-engine

# graphql-engine needs some waiting time before it's up and working
sleep 10

cd packages/admin-portal
gq http://graphql-engine:8080/v1/graphql \
    -H 'X-Hasura-Admin-Secret: admin' \
    --introspect  \
    --format json \
    > graphql.schema.json
cd ..
cp admin-portal/graphql.schema.json windmill/src/graphql/schema.json
yarn generate:admin-portal

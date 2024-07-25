#!/bin/bash -i
# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only


ENV_FILE="/workspaces/step/packages/sequent-cli/.env.example"

# Check if the .env file exists
if [ -f "$ENV_FILE" ]; then
    # Source the .env file to load environment variables
    set -a
    source "$ENV_FILE"
    set +a

   if [ -z "$TENANT_ID" ] || [ -z "$HASURA_ENDPOINT" ] || [ -z "$KEYCLOAK_URL" ] || [ -z "$KEYCLOAK_USER" ] || [ -z "$KEYCLOAK_PASSWORD" ] || [ -z "$KEYCLOAK_CLIENT_ID" ] || [ -z "$KEYCLOAK_CLIENT_SECRET" ]; then
        echo "missing default environments for auto config"
    else
       sequent config --tenant-id "$TENANT_ID" --endpoint-url "$HASURA_ENDPOINT" --keycloak-url "$KEYCLOAK_URL" --keycloak-user "$KEYCLOAK_USER" --keycloak-password "$KEYCLOAK_PASSWORD" --keycloak-client-id "$KEYCLOAK_CLIENT_ID" --keycloak-client-secret "$KEYCLOAK_CLIENT_SECRET"
    fi
else
    echo ".env file not found."
fi
#!/bin/bash -i
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

if [ -z "$SUPER_ADMIN_TENANT_ID" ] \
    || [ -z "$HASURA_ENDPOINT" ] \
    || [ -z "$KEYCLOAK_URL" ] \
    || [ -z "$KEYCLOAK_ADMIN" ] \
    || [ -z "$KEYCLOAK_CLI_CLIENT_ID" ] \
    || [ -z "$KEYCLOAK_CLI_CLIENT_SECRET" ]
then
    echo "missing default environments for auto config"
else
    step-cli step config \
        --tenant-id "$SUPER_ADMIN_TENANT_ID" \
        --endpoint-url "$HASURA_ENDPOINT" \
        --keycloak-url "$KEYCLOAK_URL" \
        --keycloak-user "$KEYCLOAK_ADMIN" \
        --keycloak-password "$KEYCLOAK_ADMIN" \
        --keycloak-client-id "$KEYCLOAK_CLI_CLIENT_ID" \
        --keycloak-client-secret "$KEYCLOAK_CLI_CLIENT_SECRET"
fi

if [ $? -ne 0 ]; then
    echo "Failed to configure step-cli"
else
    echo "step-cli configured successfully"
fi

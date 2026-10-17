#!/bin/bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# The realm template carries no passwords (meta#13842), so a fresh
# devcontainer's tenant admin and trustees cannot sign in. Give them the local
# development passwords that config-cli.sh and the dev tooling use. Accounts
# that already have a password are left as they are.
#
# Development only: sourced by init-cli.sh after .devcontainer/.env, it uses the
# local Keycloak bootstrap admin.

dev_admin_token() {
    curl -sf "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" \
        -d grant_type=password -d client_id=admin-cli \
        --data-urlencode "username=$KEYCLOAK_ADMIN" \
        --data-urlencode "password=$KEYCLOAK_ADMIN_PASSWORD" |
        jq -er .access_token
}

# set_dev_password ADMIN_API TOKEN USERNAME PASSWORD
set_dev_password() {
    local api=$1 token=$2 username=$3 password=$4 user id
    user=$(curl -sf -H "Authorization: Bearer $token" \
        "$api/users?exact=true&username=$username" | jq -c '.[0] // empty') || return 1
    if [ -z "$user" ]; then
        echo "No $username in the realm, skipped"
        return 0
    fi
    id=$(jq -r .id <<<"$user")
    if curl -sf -H "Authorization: Bearer $token" "$api/users/$id/credentials" |
        jq -e 'any(.[]; .type == "password")' >/dev/null; then
        echo "$username already has a password"
        return 0
    fi
    jq -n --arg value "$password" '{type: "password", value: $value, temporary: false}' |
        curl -sf -X PUT -H "Authorization: Bearer $token" -H 'Content-Type: application/json' \
            --data @- "$api/users/$id/reset-password" || return 1
    # The template asks the admin to choose a password, which blocks the CLI's
    # password grant.
    jq '.requiredActions = (.requiredActions // [] | map(select(. != "UPDATE_PASSWORD")))' <<<"$user" |
        curl -sf -X PUT -H "Authorization: Bearer $token" -H 'Content-Type: application/json' \
            --data @- "$api/users/$id" || return 1
    echo "Set the development password of $username"
}

set_dev_passwords() {
    local api="$KEYCLOAK_URL/admin/realms/tenant-$SUPER_ADMIN_TENANT_ID" token="" _
    # Keycloak imports the realm on its first start.
    for _ in $(seq 60); do
        token=$(dev_admin_token 2>/dev/null) &&
            curl -sf -o /dev/null -H "Authorization: Bearer $token" "$api" && break
        token=""
        sleep 5
    done
    if [ -z "$token" ]; then
        echo "$api is not reachable"
        return 1
    fi
    # config-cli.sh signs in as $KEYCLOAK_ADMIN with that same value as password.
    set_dev_password "$api" "$token" "$KEYCLOAK_ADMIN" "$KEYCLOAK_ADMIN" &&
        set_dev_password "$api" "$token" trustee1 "${TRUSTEE1_PASSWORD:-trustee1}" &&
        set_dev_password "$api" "$token" trustee2 "${TRUSTEE2_PASSWORD:-trustee2}"
}

set_dev_passwords || echo "Failed to set the development passwords"

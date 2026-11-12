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

# The bootstrap password goes through stdin, so it never shows in the
# process arguments.
dev_admin_token() {
    printf '%s' "$KEYCLOAK_ADMIN_PASSWORD" |
        curl -sf "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" \
            -d grant_type=password -d client_id=admin-cli \
            --data-urlencode "username=$KEYCLOAK_ADMIN" \
            --data-urlencode password@- |
        jq -er .access_token
}

# kc_curl TOKEN CURL_ARGS...: curl with the bearer token in a header read from
# a file descriptor, so the token never shows in the process arguments.
kc_curl() {
    local token=$1
    shift
    curl -sf -H @<(printf 'Authorization: Bearer %s\n' "$token") "$@"
}

# set_dev_password ADMIN_API TOKEN USERNAME PASSWORD
set_dev_password() {
    local api=$1 token=$2 username=$3 password=$4 users user id credentials
    users=$(kc_curl "$token" "$api/users?exact=true&username=$username") || return 1
    user=$(jq -c '.[0] // empty' <<<"$users") || return 1
    if [ -z "$user" ]; then
        echo "No $username in the realm, skipped"
        return 0
    fi
    id=$(jq -r .id <<<"$user")
    credentials=$(kc_curl "$token" "$api/users/$id/credentials") || return 1
    if jq -e 'any(.[]; .type == "password")' >/dev/null <<<"$credentials"; then
        echo "$username already has a password"
    else
        jq -n --arg value "$password" '{type: "password", value: $value, temporary: false}' |
            kc_curl "$token" -X PUT -H 'Content-Type: application/json' \
                --data @- "$api/users/$id/reset-password" || return 1
        echo "Set the development password of $username"
    fi
    # The template asks the admin to choose a password, which blocks the CLI's
    # password grant. This also runs for an account that already has a
    # password, so a run that stopped after the reset is completed.
    if jq -e '.requiredActions // [] | index("UPDATE_PASSWORD")' >/dev/null <<<"$user"; then
        jq '.requiredActions |= map(select(. != "UPDATE_PASSWORD"))' <<<"$user" |
            kc_curl "$token" -X PUT -H 'Content-Type: application/json' \
                --data @- "$api/users/$id" || return 1
    fi
}

set_dev_passwords() {
    local api="$KEYCLOAK_URL/admin/realms/tenant-$SUPER_ADMIN_TENANT_ID" token="" _
    # Keycloak imports the realm on its first start.
    for _ in $(seq 60); do
        token=$(dev_admin_token 2>/dev/null) &&
            kc_curl "$token" -o /dev/null "$api" && break
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

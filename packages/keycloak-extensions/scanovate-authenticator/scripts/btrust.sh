#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
#
# Points the Scanovate B-Trust steps of a realm to a B-Trust environment (the
# mock server, the Scanovate test environment or production), and fetches the
# results of a B-Trust session to check the configured rules against them.
#
# Usage:
#   btrust.sh configure <realm>
#   btrust.sh results <processId>
#
# Environment:
#   SCANOVATE_BASE_URL, SCANOVATE_CLIENT_ID, SCANOVATE_CLIENT_SECRET: B-Trust
#     API and credentials. Required.
#   SCANOVATE_FLOW_ID: B-Trust flow to launch. Required by configure.
#   SCANOVATE_EXECUTION_MODE: interactive (default) or auto-complete.
#   SCANOVATE_SAVE_OPTION: empty (default), save or do_not_save.
#   KEYCLOAK_URL: Keycloak base URL, default http://127.0.0.1:8090.
#   KEYCLOAK_ADMIN, KEYCLOAK_ADMIN_PASSWORD: master realm admin, default admin/admin.

set -euo pipefail

KEYCLOAK_URL="${KEYCLOAK_URL:-http://127.0.0.1:8090}"
KEYCLOAK_ADMIN="${KEYCLOAK_ADMIN:-admin}"
KEYCLOAK_ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD:-admin}"
AUTHENTICATOR_ID="scanovate-authenticator"

usage() {
  sed -n '/^# Usage:/,/^$/p' "$0" | sed 's/^# \{0,1\}//'
  exit 1
}

require() {
  local name
  for name in "$@"; do
    if [ -z "${!name:-}" ]; then
      echo "error: $name is not set" >&2
      exit 1
    fi
  done
}

keycloak_admin_token() {
  curl -sSf "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" \
    --data-urlencode "grant_type=password" \
    --data-urlencode "client_id=admin-cli" \
    --data-urlencode "username=$KEYCLOAK_ADMIN" \
    --data-urlencode "password=$KEYCLOAK_ADMIN_PASSWORD" | jq -r .access_token
}

configure() {
  local realm="$1"
  require SCANOVATE_BASE_URL SCANOVATE_CLIENT_ID SCANOVATE_CLIENT_SECRET SCANOVATE_FLOW_ID
  local token admin config_ids config_id config updated
  token="$(keycloak_admin_token)"
  admin="$KEYCLOAK_URL/admin/realms/$realm/authentication"

  config_ids="$(
    curl -sSf -H "Authorization: Bearer $token" "$admin/flows" \
      | jq -r '.[] | select(.topLevel) | .alias | @uri' \
      | while read -r flow; do
          curl -sSf -H "Authorization: Bearer $token" "$admin/flows/$flow/executions" \
            | jq -r --arg id "$AUTHENTICATOR_ID" \
              '.[] | select(.providerId == $id and .authenticationConfig != null) | .authenticationConfig'
        done | sort -u
  )"
  if [ -z "$config_ids" ]; then
    echo "error: no configured $AUTHENTICATOR_ID step in realm $realm" >&2
    exit 1
  fi

  for config_id in $config_ids; do
    config="$(curl -sSf -H "Authorization: Bearer $token" "$admin/config/$config_id")"
    updated="$(
      jq \
        --arg base_url "$SCANOVATE_BASE_URL" \
        --arg client_id "$SCANOVATE_CLIENT_ID" \
        --arg client_secret "$SCANOVATE_CLIENT_SECRET" \
        --arg flow_id "$SCANOVATE_FLOW_ID" \
        --arg execution_mode "${SCANOVATE_EXECUTION_MODE:-interactive}" \
        --arg save_option "${SCANOVATE_SAVE_OPTION:-}" \
        '.config += {
          "base-url": $base_url,
          "client-id": $client_id,
          "client-secret": $client_secret,
          "flow-id": $flow_id,
          "execution-mode": $execution_mode,
          "save-option": $save_option
        }' <<<"$config"
    )"
    curl -sSf -X PUT -H "Authorization: Bearer $token" -H "Content-Type: application/json" \
      --data "$updated" "$admin/config/$config_id"
    echo "Updated $(jq -r .alias <<<"$config") in $realm: $SCANOVATE_BASE_URL, flow $SCANOVATE_FLOW_ID, ${SCANOVATE_EXECUTION_MODE:-interactive}"
  done
}

results() {
  local process_id="$1"
  require SCANOVATE_BASE_URL SCANOVATE_CLIENT_ID SCANOVATE_CLIENT_SECRET
  local access_token session_token
  access_token="$(
    curl -sSf -H "Content-Type: application/json" "$SCANOVATE_BASE_URL/auth/token" \
      --data "$(jq -n --arg id "$SCANOVATE_CLIENT_ID" --arg secret "$SCANOVATE_CLIENT_SECRET" \
        '{client_id: $id, client_secret: $secret}')" \
      | jq -r .access_token
  )"
  session_token="$(
    curl -sSf -H "Authorization: Bearer $access_token" \
      "$SCANOVATE_BASE_URL/api/v3/mobile_interaction/$process_id/token" | jq -r .token
  )"
  curl -sSf -H "Authorization: Bearer $session_token" \
    "$SCANOVATE_BASE_URL/api/v3/mobile_interaction/v2/$session_token/results_with_image_names" | jq .
}

[ $# -eq 2 ] || usage
case "$1" in
  configure) configure "$2" ;;
  results) results "$2" ;;
  *) usage ;;
esac

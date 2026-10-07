#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
#
# Points the Scanovate identity verification steps of a realm to the Scanovate
# services hosted on premise (OCR, Liveness Plus and Face Match), and removes
# the settings of the B-Trust cloud integration that preceded them.
#
# Usage:
#   configure-realm.sh configure <realm>
#
# Environment:
#   SCANOVATE_OCR_URL: OCR URL as Keycloak reaches it. Required.
#   SCANOVATE_LIVENESS_URL, SCANOVATE_LIVENESS_SECRET, SCANOVATE_FACE_MATCH_URL:
#     Liveness Plus URL as the browser reaches it, its callback secret, and
#     the Face Match URL as Keycloak reaches it. Required.
#   SCANOVATE_OCR_TYPES: ocr-types, default {"default": "passport"}.
#   SCANOVATE_FACE_MATCH_MIN_SIMILARITY: face-match-min-similarity, default
#     {"default": 0.67}.
#   SCANOVATE_CAPTURE_SIDES: capture-sides, default the passport's front only
#     and both sides of any other document.
#   SCANOVATE_LOGIN_THEME: if set, the login theme of the realm, and of its
#     clients that set their own, e.g. sequent-ui-voting, which has the
#     capture page.
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

# Prints the ids of the configs of every Scanovate step of the realm.
scanovate_config_ids() {
  local token="$1" admin="$2" realm="$3" config_ids
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
  echo "$config_ids"
}

configure() {
  local realm="$1"
  require SCANOVATE_OCR_URL SCANOVATE_LIVENESS_URL SCANOVATE_LIVENESS_SECRET SCANOVATE_FACE_MATCH_URL
  local token admin config_ids config_id config updated
  token="$(keycloak_admin_token)"
  admin="$KEYCLOAK_URL/admin/realms/$realm/authentication"
  config_ids="$(scanovate_config_ids "$token" "$admin" "$realm")"

  for config_id in $config_ids; do
    config="$(curl -sSf -H "Authorization: Bearer $token" "$admin/config/$config_id")"
    updated="$(
      jq \
        --arg ocr_url "$SCANOVATE_OCR_URL" \
        --arg ocr_types "${SCANOVATE_OCR_TYPES:-{\"default\": \"passport\"\}}" \
        --arg liveness_url "$SCANOVATE_LIVENESS_URL" \
        --arg liveness_secret "$SCANOVATE_LIVENESS_SECRET" \
        --arg face_match_url "$SCANOVATE_FACE_MATCH_URL" \
        --arg min_similarity "${SCANOVATE_FACE_MATCH_MIN_SIMILARITY:-{\"default\": 0.67\}}" \
        --arg capture_sides "${SCANOVATE_CAPTURE_SIDES:-{\"philippinePassport\": [\"front\"], \"default\": [\"front\", \"back\"]\}}" \
        '.config |= (
          del(."base-url", ."client-id", ."client-secret", ."flow-id", ."execution-mode",
              ."save-option", ."link-params", ."doc-id", ."face-capture", ."max-retries",
              ."max-video-bytes")
          + {
            "ocr-url": $ocr_url,
            "ocr-types": $ocr_types,
            "liveness-url": $liveness_url,
            "liveness-secret": $liveness_secret,
            "face-match-url": $face_match_url,
            "face-match-min-similarity": $min_similarity,
            "capture-sides": $capture_sides
          })' <<<"$config"
    )"
    curl -sSf -X PUT -H "Authorization: Bearer $token" -H "Content-Type: application/json" \
      --data "$updated" "$admin/config/$config_id"
    echo "Updated $(jq -r .alias <<<"$config") in $realm: $SCANOVATE_OCR_URL, $SCANOVATE_LIVENESS_URL, $SCANOVATE_FACE_MATCH_URL"
    # Rules written for B-Trust read processes that no longer exist and fail every voter
    if jq -e '[.config["attributes-to-validate"], .config["attributes-to-store"]]
        | map(select(. != null and . != "") | fromjson | .[][] | .process // empty)
        | any(. != "ocr" and . != "authentications")' <<<"$updated" >/dev/null; then
      echo "warning: $(jq -r .alias <<<"$config") has rules on processes other than ocr and authentications; rewrite them for the OCR results" >&2
    fi
  done

  if [ -n "${SCANOVATE_LOGIN_THEME:-}" ]; then
    curl -sSf -X PUT -H "Authorization: Bearer $token" -H "Content-Type: application/json" \
      --data "$(jq -n --arg theme "$SCANOVATE_LOGIN_THEME" '{loginTheme: $theme}')" \
      "$KEYCLOAK_URL/admin/realms/$realm"
    echo "Set the login theme of $realm to $SCANOVATE_LOGIN_THEME"
    # A client's own login theme overrides the realm's for its logins
    local client client_id
    curl -sSf -H "Authorization: Bearer $token" "$KEYCLOAK_URL/admin/realms/$realm/clients" \
      | jq -c '.[] | select((.attributes.login_theme // "") != "")' \
      | while read -r client; do
          client_id="$(jq -r .id <<<"$client")"
          curl -sSf -X PUT -H "Authorization: Bearer $token" -H "Content-Type: application/json" \
            --data "$(jq --arg theme "$SCANOVATE_LOGIN_THEME" '.attributes.login_theme = $theme' <<<"$client")" \
            "$KEYCLOAK_URL/admin/realms/$realm/clients/$client_id"
          echo "Set the login theme of client $(jq -r .clientId <<<"$client") to $SCANOVATE_LOGIN_THEME"
        done
  fi
}

[ $# -eq 2 ] || usage
case "$1" in
  configure) configure "$2" ;;
  *) usage ;;
esac

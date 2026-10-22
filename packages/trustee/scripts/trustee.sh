#!/bin/bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail
umask 077

cd /opt/trustee
TRUSTEE_CONFIG_PATH=${TRUSTEE_CONFIG_PATH:-/opt/trustee/trustee.toml}
SECRETS_BACKEND=${SECRETS_BACKEND:-Awssecretsmanager}

if [[ ! -f "$TRUSTEE_CONFIG_PATH" ]]; then
    secret_name="secrets/${TRUSTEE_NAME:?TRUSTEE_NAME is required when fetching keys}_config"
    case "${SECRETS_BACKEND,,}" in
        envvarmastersecret)
            config_content=${TRUSTEE_CONFIG:-}
            ;;
        awssecretsmanager|awssecretmanager)
            secret_name="${AWS_SM_KEY_PREFIX:?AWS_SM_KEY_PREFIX is required}${secret_name}"
            config_content=$(aws secretsmanager get-secret-value --secret-id "$secret_name" --query SecretString --output text)
            ;;
        hashicorpvault)
            export VAULT_ADDR=${VAULT_SERVER_URL:?VAULT_SERVER_URL is required}
            export VAULT_TOKEN=${VAULT_TOKEN:?VAULT_TOKEN is required}
            config_content=$(vault kv get -field=value "$secret_name")
            ;;
        *)
            echo "Unsupported SECRETS_BACKEND" >&2
            exit 1
            ;;
    esac
    if [[ -z "$config_content" ]]; then
        echo "No trustee keys found. Run trustee generate, back up the keys file and register both public keys before starting." >&2
        exit 1
    fi
    # A failed write never replaces an existing keys file.
    (set -o noclobber; printf '%s\n' "$config_content" > "$TRUSTEE_CONFIG_PATH")
    unset config_content
fi

exec trustee start --b4-url "${B4_URL:?B4_URL is required: the Harvest board route, e.g. http://harvest:8400/b4}" --trustee-config "$TRUSTEE_CONFIG_PATH"

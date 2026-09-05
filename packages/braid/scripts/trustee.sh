#!/bin/bash

# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -e
umask 077

# Set default values
cd /opt/braid
#bb_helper --cache-dir /tmp/cache -s "$IMMUDB_URL" -b defaultboard -u "$IMMUDB_USER" -p "$IMMUDB_PASSWORD" upsert-board-db -l debug
TRUSTEE_CONFIG_PATH=${TRUSTEE_CONFIG_PATH:-"/opt/braid/trustee.toml"} # Skipping secretsService if TRUSTEE_CONFIG_PATH is set
SECRETS_BACKEND=${SECRETS_BACKEND:-"EnvVarMasterSecret"}
SECRETS_BACKEND_LOWER=$(echo "$SECRETS_BACKEND" | tr '[:upper:]' '[:lower:]')
if [ -z "$TRUSTEE_NAME" ] && [ ! -f "$TRUSTEE_CONFIG_PATH" ]; then
    echo "Error: TRUSTEE_NAME must be set." #Avoid secrets overwriting
    exit 1
fi

# Check if the binary exists
if ! command -v gen_trustee_config &> /dev/null; then
    echo "Error: gen_trustee_config binary not found in PATH"
    exit 1
fi

SECRET_KEY_NAME="secrets/${TRUSTEE_NAME}_config"

if [ "$SECRETS_BACKEND_LOWER" = "awssecretmanager" ]; then
    SECRETS_BACKEND_LOWER="awssecretsmanager"
fi

if [ "$SECRETS_BACKEND_LOWER" = "awssecretsmanager" ]; then
    if [ -z "$AWS_SM_KEY_PREFIX" ] && [ ! -f "$TRUSTEE_CONFIG_PATH" ]
    then
        echo "Error: AWS_SM_KEY_PREFIX must be set." #Avoid secrets overwriting
        exit 1
    fi
    SECRET_KEY_NAME="${AWS_SM_KEY_PREFIX}${SECRET_KEY_NAME}"
fi

# Fail before generating or changing a trustee key when an optional backend is unavailable.
# An explicitly supplied configuration file does not need a cloud secret service.
if [ ! -f "$TRUSTEE_CONFIG_PATH" ]; then
    case "$SECRETS_BACKEND_LOWER" in
        awssecretsmanager|hashicorpvault)
            if [ "${CLOUD_SECRET_BACKENDS:-disabled}" != "enabled" ]; then
                echo "Error: cloud secret backends are disabled in this image configuration." >&2
                exit 1
            fi
            if [ "$SECRETS_BACKEND_LOWER" = "awssecretsmanager" ]; then
                command -v aws >/dev/null || { echo "Error: reviewed AWS CLI input missing." >&2; exit 1; }
            else
                # OpenBao implements the Vault KV interface used below.
                VAULT_CLI=${VAULT_CLI:-bao}
                command -v "$VAULT_CLI" >/dev/null || { echo "Error: reviewed Vault-compatible CLI input missing." >&2; exit 1; }
            fi
            ;;
    esac
fi

# Export Vault-compatible client environment variables
export VAULT_ADDR="${VAULT_SERVER_URL:-${VAULT_ADDR:-}}"
export VAULT_TOKEN="${VAULT_TOKEN:-}"

# Function to log messages
log() {
    echo "$(date +"%Y-%m-%d %H:%M:%S") - $1"
}

# Fetch secret from AWS Secrets Manager
fetch_secret_aws() {
    aws secretsmanager get-secret-value --secret-id "$1" --query 'SecretString' --output text
}

# Fetch secret from HashiCorp Vault
fetch_secret_vault() {
    "$VAULT_CLI" kv get -field=value "$1"
}

# Store secret in AWS Secrets Manager
store_secret_aws() {
    aws secretsmanager create-secret --name "$1" --secret-string "$2"
}

# Store secret in HashiCorp Vault
store_secret_vault() {
    "$VAULT_CLI" kv put "$1" value="$2"
}

# Main function to handle the config
handle_trustee_config() {
    local config_content
    log "Querying secrets service for config..."

    if [ -f "$TRUSTEE_CONFIG_PATH" ]; then
        config_content=$(<"$TRUSTEE_CONFIG_PATH")
        log "Using existing config from $TRUSTEE_CONFIG_PATH"
    else
        case "$SECRETS_BACKEND_LOWER" in
            "envvarmastersecret")
                if [ -z "$TRUSTEE_CONFIG" ]; then
                    if [ "${TRUSTEE_ALLOW_EPHEMERAL:-false}" != "true" ]; then
                        echo "Error: provide a persistent trustee configuration file or TRUSTEE_CONFIG." >&2
                        exit 1
                    fi
                    log "Explicit test mode: generating ephemeral config"
                    config_content=$(gen_trustee_config)
                else
                    config_content=$TRUSTEE_CONFIG
                fi
                ;;
            "awssecretsmanager")
                config_content=$(fetch_secret_aws "$SECRET_KEY_NAME" 2>/dev/null) || {
                    log "Failed to fetch from AWS Secrets Manager; no key created or replaced"
                    exit 1
                }
                ;;
            "hashicorpvault")
                config_content=$(fetch_secret_vault "$SECRET_KEY_NAME" 2>/dev/null) || {
                    log "Failed to fetch from Vault-compatible service; no key created or replaced"
                    exit 1
                }
                ;;
            *)
                echo "Error: Unsupported SECRETS_BACKEND: $SECRETS_BACKEND"
                exit 1
                ;;
        esac

        if [ -z "$config_content" ]; then
            echo "Error: retrieved trustee configuration is empty; provision it explicitly before startup." >&2
            exit 1
        fi
    fi

    if [ ! -f "$TRUSTEE_CONFIG_PATH" ] || [ "$(cat "$TRUSTEE_CONFIG_PATH")" != "$config_content" ]; then
        printf "%s\n" "$config_content" > "$TRUSTEE_CONFIG_PATH"
        log "Wrote config to $TRUSTEE_CONFIG_PATH"
    fi
    grep key_pk "$TRUSTEE_CONFIG_PATH"
}

handle_trustee_config

# Run trustee with the generated or fetched config
trustee --b4-url "$B4_URL" --trustee-config "$TRUSTEE_CONFIG_PATH"

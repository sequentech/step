#!/bin/bash

# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -e
set -x

# Set default values
cd /opt/braid
#bb_helper --cache-dir /tmp/cache -s "$IMMUDB_URL" -b defaultboard -u "$IMMUDB_USER" -p "$IMMUDB_PASSWORD" upsert-board-db -l debug
TRUSTEE_CONFIG_PATH=${TRUSTEE_CONFIG_PATH:-"/opt/braid/trustee.toml"} # Skipping secretsService if TRUSTEE_CONFIG_PATH is set
SECRETS_BACKEND=${SECRETS_BACKEND:-"Awssecretsmanager"} # Default to Awssecretsmanager if not set
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

# Export Vault environment variables (Consumed internally by vault binary)
export VAULT_ADDR="${VAULT_SERVER_URL}"
export VAULT_TOKEN="${VAULT_TOKEN}"

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
    vault kv get -field=value "$1"
}

# Store secret in AWS Secrets Manager
store_secret_aws() {
    aws secretsmanager create-secret --name "$1" --secret-string "$2"
}

# Store secret in HashiCorp Vault
store_secret_vault() {
    vault kv put "$1" value="$2"
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
                    log "TRUSTEE_CONFIG empty, generating ephemeral config"
                    config_content=$(gen_trustee_config)
                else
                    config_content=$(echo -e "$TRUSTEE_CONFIG")
                fi
                ;;
            "awssecretsmanager")
                config_content=$(fetch_secret_aws "$SECRET_KEY_NAME" 2>/dev/null) || {
                    log "Failed to fetch from AWS Secrets Manager"
                    config_content=""
                }
                ;;
            "hashicorpvault")
                config_content=$(fetch_secret_vault "$SECRET_KEY_NAME" 2>/dev/null) || {
                    log "Failed to fetch from HashiCorp Vault"
                    config_content=""
                }
                ;;
            *)
                echo "Error: Unsupported SECRETS_BACKEND: $SECRETS_BACKEND"
                exit 1
                ;;
        esac

        if [ -z "$config_content" ]; then
            log "Config does not exist, generating..."
            config_content=$(gen_trustee_config)
            if [ "$SECRETS_BACKEND_LOWER" = "awssecretsmanager" ]; then
                store_secret_aws "$SECRET_KEY_NAME" "$config_content"
            elif [ "$SECRETS_BACKEND_LOWER" = "hashicorpvault" ]; then
                store_secret_vault "$SECRET_KEY_NAME" "$config_content"
            fi
        fi
    fi

    if [ ! -f "$TRUSTEE_CONFIG_PATH" ] || [ "$(cat "$TRUSTEE_CONFIG_PATH")" != "$config_content" ]; then
        printf "%b" "$config_content" > "$TRUSTEE_CONFIG_PATH"
        log "Wrote config to $TRUSTEE_CONFIG_PATH"
    fi
    grep key_pk "$TRUSTEE_CONFIG_PATH"
}

handle_trustee_config

# Run trustee with the generated or fetched config
trustee --b3-url "$B3_URL" --trustee-config "$TRUSTEE_CONFIG_PATH"

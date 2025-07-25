#!/bin/bash

# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -e
set -x

# Set default values
cd /opt/braid
#bb_helper --cache-dir /tmp/cache -s "$IMMUDB_URL" -b defaultboard -u "$IMMUDB_USER" -p "$IMMUDB_PASSWORD" upsert-board-db -l debug
TRUSTEE_CONFIG_PATH=${TRUSTEE_CONFIG:-"/opt/braid/trustee.toml"} # Skipping secretsService if TRUSTEE_CONFIG is set
SECRETS_BACKEND=${SECRETS_BACKEND:-"awsSecretsManager"} # Default to awsSecretsManager if not set
if [ -z "$TRUSTEE_NAME" ] && [ ! -f "$TRUSTEE_CONFIG_PATH" ]; then
    echo "Error: TRUSTEE_NAME must be set." #Avoid secrets overwriting
    exit 1
fi
SECRET_KEY_NAME="secrets/${TRUSTEE_NAME}_config"

if [ "$SECRETS_BACKEND" = "awsSecretsManager" ]; then
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

    if [ -f $TRUSTEE_CONFIG_PATH ]; then
        TRUSTEE_CONFIG_DATA=$(<$TRUSTEE_CONFIG_PATH);
        config_content=$TRUSTEE_CONFIG_DATA
    fi

    # Fetch config if awsSecretsManager or hashicorpVault
    if [ "$SECRETS_BACKEND" = "awsSecretsManager" ]; then
        config_content=$(fetch_secret_aws "$SECRET_KEY_NAME" 2>/dev/null) || true
    fi

    if [ "$SECRETS_BACKEND" = "hashicorpVault" ]; then
        config_content=$(fetch_secret_vault "$SECRET_KEY_NAME" 2>/dev/null) || true
    fi

    if [ -z "$config_content" ]; then
        log "Config does not exist, generating..."
        if [ -z "$TRUSTEE_CONFIG_DATA" ]; then
            config_content=$(gen_trustee_config)
        else
            config_content=$TRUSTEE_CONFIG_DATA
        fi
        
        if [ "$SECRETS_BACKEND" = "awsSecretsManager" ]; then
            store_secret_aws "$SECRET_KEY_NAME" "$config_content"
        else
            store_secret_vault "$SECRET_KEY_NAME" "$config_content"
        fi
    else
        log "Config exists, using existing configuration"
    fi
    if [ -z "$TRUSTEE_CONFIG_DATA" ]; then
        echo "$config_content" > "$TRUSTEE_CONFIG_PATH"
    fi
    cat "$TRUSTEE_CONFIG_PATH"
}

handle_trustee_config

# Run trustee with the generated or fetched config
trustee --b3-url "$B3_URL" --trustee-config "$TRUSTEE_CONFIG_PATH"

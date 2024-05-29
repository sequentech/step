#!/bin/sh
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# entrypoint.sh

# Path to the JSON file
JSON_FILE="/usr/share/nginx/html/global-settings.json"

# Iterate over each key in the JSON file
for key in $(jq -r 'keys[]' $JSON_FILE); do
    # Construct the environment variable name from the JSON key
    # Convert JSON key format to uppercase environment variable name (e.g., default_tenant_id -> DEFAULT_TENANT_ID)
    env_var_name=$(echo $key | awk '{print toupper($0)}' | tr '-' '_')

    # Fetch the value of the environment variable
    env_var_value=$(printenv $env_var_name)

    # If the environment variable exists and is not empty
    if [ ! -z "$env_var_value" ]; then
        # Update the value in the JSON file
        jq --arg key "$key" --arg value "$env_var_value" '.[$key] = $value' $JSON_FILE > /tmp/global-settings.json && mv /tmp/global-settings.json $JSON_FILE
    fi
done

# Start NGINX in foreground mode
exec nginx -g "daemon off;"
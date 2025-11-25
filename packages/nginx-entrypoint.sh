#!/bin/sh
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# entrypoint.sh

# Path to the JSON file
JSON_FILE="/usr/share/nginx/html/global-settings.json"
TEMP_FILE="/tmp/global-settings.json"

# Function to detect and convert value type
convert_value() {
    local value="$1"
    local original_type="$2"

    # If original was a number, try to parse as number
    if [ "$original_type" = "number" ]; then
        # Check if value is a valid number
        if echo "$value" | grep -qE '^-?[0-9]+(\.[0-9]+)?$'; then
            # Return as number (remove quotes)
            echo "$value"
            return
        fi
    fi

    # If original was boolean or value looks like boolean
    if [ "$original_type" = "boolean" ] || [ "$value" = "true" ] || [ "$value" = "false" ]; then
        case "$value" in
            "true"|"1"|"yes"|"on") echo "true" ;;
            "false"|"0"|"no"|"off") echo "false" ;;
            *) echo "\"$value\"" ;;  # Fallback to string if invalid boolean
        esac
        return
    fi

    # Default: return as quoted string
    echo "\"$value\""
}

# Function to get the type of a JSON value
get_json_type() {
    local key="$1"
    jq -r --arg key "$key" 'if has($key) then (.[$key] | type) else "null" end' "$JSON_FILE"
}

# Process simple top-level keys first
for key in $(jq -r 'keys[]' "$JSON_FILE"); do
    # Construct the environment variable name from the JSON key
    env_var_name=$(echo "$key" | awk '{print toupper($0)}' | tr '-' '_')

    # Fetch the value of the environment variable
    env_var_value=$(printenv "$env_var_name")

    # If the environment variable exists and is not empty
    if [ ! -z "$env_var_value" ]; then
        # Get the original type of the JSON value
        original_type=$(get_json_type "$key")

        # Check if the environment variable value is valid JSON
        if echo "$env_var_value" | jq . >/dev/null 2>&1; then
            # It's valid JSON, use it directly with --argjson
            jq --arg key "$key" \
               --argjson value "$env_var_value" \
               '.[$key] = $value' "$JSON_FILE" > "$TEMP_FILE" && mv "$TEMP_FILE" "$JSON_FILE"
        else
            # Convert the environment variable value to appropriate type
            converted_value=$(convert_value "$env_var_value" "$original_type")

            # Update the value in the JSON file with proper type
            jq --arg key "$key" \
               --argjson value "$converted_value" \
               '.[$key] = $value' "$JSON_FILE" > "$TEMP_FILE" && mv "$TEMP_FILE" "$JSON_FILE"
        fi
    fi
done

# Start NGINX in foreground mode
exec nginx -g "daemon off;"
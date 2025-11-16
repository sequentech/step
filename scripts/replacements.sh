#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

replace-keycloak-data() {
    # Usage:
    #   replace-keycloak-data [REPLACEMENTS_JSON] [TARGET_FILE]
    #
    # If not provided, default paths will be used.

    # 1st argument = the JSON with replacements, or the default
    local REPLACEMENTS="${1:-/workspaces/step/packages/windmill/external-bin/janitor/config/beyond.json}"
    # 2nd argument = the file to modify, or the default
    local TENANT_FILE="${2:-/workspaces/step/.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json}"

    echo "Using replacements from: $REPLACEMENTS"
    echo "Modifying file: $TENANT_FILE"
    echo
    
    # Read the keys line by line instead of splitting on spaces
    while IFS= read -r key; do
    # Extract the corresponding value for this key
    val=$(jq -r ".replacements[\"$key\"]" "$REPLACEMENTS")

    # Escape sed-special characters in both key and value
    key_escaped=$(printf '%s\n' "$key" | sed -E 's/[]\/$*.^|[]/\\&/g')
    val_escaped=$(printf '%s\n' "$val" | sed -E 's/[]\/$*.^|[]/\\&/g')

    echo "Replacing '$key' with '$val' in $TENANT_FILE"

    # Create a small temporary sed script for this replacement
    cat <<EOF > replace.sed
s~$key_escaped~$val_escaped~g
EOF

    # Apply the script so we don’t pass huge strings directly to sed’s -e
    sed -i -f replace.sed "$TENANT_FILE"

    done < <(jq -r '.replacements | keys[]' "$REPLACEMENTS")  # feed keys to the while loop

    # Clean up
    rm -f replace.sed
}

# Call the function with all script arguments, so you can run:
#   ./replacements.sh [replacements.json] [tenant_file.json]
replace-keycloak-data "$@"
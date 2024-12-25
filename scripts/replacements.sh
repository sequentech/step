#!/usr/bin/env bash

replace-keycloak-data() {
    # JSON with replacements
    REPLACEMENTS="/workspaces/step/packages/windmill/external-bin/janitor/config/baseConfig.json"
    # File to modify
    TENANT_FILE="/workspaces/step/.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json" 

    # Loop over each key in .replacements
    for key in $(jq -r '.replacements | keys[]' "$REPLACEMENTS"); do
        val=$(jq -r ".replacements[\"$key\"]" "$REPLACEMENTS")

        # Escape characters that are special in sed (/, ., *, $, ^, [, ], &)
        key_escaped=$(printf '%s\n' "$key" | sed -E 's/[]\/$*.^|[]/\\&/g')
        val_escaped=$(printf '%s\n' "$val" | sed -E 's/[]\/$*.^|[]/\\&/g')

        echo "Replacing '$key' with '$val' in $TENANT_FILE"

        # Generate a temporary sed script for just this replacement
        cat <<EOF > replace.sed
s~$key_escaped~$val_escaped~g
EOF

        # Use the script with -f instead of passing the substitution directly
        sed -i -f replace.sed "$TENANT_FILE"

        # Remove the temporary script (optional)
        rm replace.sed
    done
}

replace-keycloak-data
#!/bin/bash -i

set -ex -o pipefail

BASE_DIR="/workspaces/backend-services"

source "${BASE_DIR}/.devcontainer/.env"

# Define the directory to search in
REALMS_DIR="${BASE_DIR}/.devcontainer/keycloak/"

# Function to apply sed replacement
apply_replacement() {
    local cmd=$1
    find "${REALMS_DIR}" -type f -exec sed -i "$cmd" {} \;
}


# Apply each replacement command
apply_replacement 's|'"${VOTING_PORTAL_URL}"'|${VOTING_PORTAL_URL}|g'
apply_replacement 's|'"${BALLOT_VERIFIER_URL}"'|${BALLOT_VERIFIER_URL}|g'
apply_replacement 's|'"${ADMIN_PORTAL_URL}"'|${ADMIN_PORTAL_URL}|g'
apply_replacement 's|'"${ONSITE_VOTING_PORTAL_URL}"'|${ONSITE_VOTING_PORTAL_URL}|g'

echo "Replacement complete."

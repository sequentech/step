#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -e

echo "Updating MinIO certificates from Keycloak realms..."

cd /workspaces/step/.devcontainer

# Clean up temporary files
[ -f /tmp/combined.json ] && rm /tmp/combined.json

# Get all realm import files
export FILES=$(ls keycloak/import/)

for FILE in $FILES; do
  # Skip files ending in .license
  [[ "$FILE" == *.license ]] && continue
  
  REALM="${FILE%.json}"
  echo "Fetching JWK for realm: ${REALM}"
  
  # Fetch the certs from Keycloak
  curl -s "http://keycloak:8090/realms/${REALM}/protocol/openid-connect/certs" | python3 -m json.tool > /tmp/certs.json
  
  # Combine with existing certs or create new file
  if [ -f /tmp/combined.json ]; then
    jq -s '{keys: (.[0].keys + .[1].keys)}' /tmp/certs.json /tmp/combined.json > /tmp/combined_new.json
    mv /tmp/combined_new.json /tmp/combined.json
  else
    cp /tmp/certs.json /tmp/combined.json
  fi
done

# Copy the combined certs to minio directory
cp /tmp/combined.json minio/certs.json

echo "Certificates updated successfully!"
echo "Certificate file contents:"

# Clean up
rm -f /tmp/certs.json /tmp/combined.json

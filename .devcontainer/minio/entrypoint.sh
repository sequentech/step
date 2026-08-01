#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -euo pipefail

upload_realm_config() {
  local source_path="$1"
  local s3_key="$2"
  local destination="myminio/${MINIO_BUCKET}/${s3_key}"

  if [[ ! -f "$source_path" ]]; then
    if mc stat "$destination" > /dev/null 2>&1; then
      echo "${source_path} is unavailable; ${destination} already exists, skipping upload..."
      return 0
    fi

    echo "Required realm config ${source_path} is unavailable and ${destination} does not exist" >&2
    return 1
  fi

  echo "Uploading ${source_path} to ${destination}..."
  mc cp "$source_path" "$destination"
}

mc alias set myminio "$MINIO_PRIVATE_URI" "$MINIO_ROOT_USER" "$MINIO_ROOT_PASSWORD"
mc mb --ignore-existing myminio/"$MINIO_PUBLIC_BUCKET"
mc mb --ignore-existing myminio/"$MINIO_BUCKET"
mc anonymous set download myminio/"$MINIO_PUBLIC_BUCKET"

if mc admin accesskey info myminio "$MINIO_ACCESS_KEY" > /dev/null 2>&1; then
  echo "MinIO access key already exists, skipping creation..."
else
  mc admin accesskey create myminio/ "$MINIO_ROOT_USER" \
    --access-key "$MINIO_ACCESS_KEY" \
    --secret-key "$MINIO_ACCESS_SECRET"
fi

echo "Uploading public-assets folder..."
mc cp --recursive /scripts/public-assets/ myminio/public/public-assets/

upload_realm_config \
  "/realm-configs/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json" \
  "$KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY"
upload_realm_config \
  "/realm-configs/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559.json" \
  "$KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY"

if mc stat myminio/public/certs.json > /dev/null 2>&1; then
  echo "certs.json already exists in MinIO, skipping upload..."
else
  echo "Uploading certs.json..."
  mc cp /scripts/certs.json myminio/public/certs.json
fi

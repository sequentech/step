#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

normalize_s3_key() {
  local variable_name="$1"
  local value="${!variable_name}"

  value="${value#"${value%%[![:space:]]*}"}"
  value="${value%"${value##*[![:space:]]}"}"

  if [[ -z "$value" ]]; then
    echo "${variable_name} must be set and not empty" >&2
    return 1
  fi
  if [[ "$value" == /* || "$value" == */ ]]; then
    echo "${variable_name} must not start or end with '/': ${value}" >&2
    return 1
  fi

  printf -v "$variable_name" '%s' "$value"
}

normalize_s3_key KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY || exit 1
normalize_s3_key KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY || exit 1

upload_realm_config() {
  local source_path="$1"
  local s3_key="$2"
  local destination="myminio/${MINIO_PUBLIC_BUCKET}/${s3_key}"

  echo "Uploading ${source_path} to ${destination}..."
  if ! mc cp "$source_path" "$destination"; then
    echo "Failed to upload ${source_path} to ${destination}" >&2
    return 1
  fi
}

if [[ "$KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY" == "$KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY" ]]; then
  echo "Tenant and election-event realm config S3 keys must be different" >&2
  exit 1
fi

mc alias set myminio "$MINIO_PRIVATE_URI" "$MINIO_ROOT_USER" "$MINIO_ROOT_PASSWORD"
mc mb -p myminio/$MINIO_PUBLIC_BUCKET
mc mb -p myminio/$MINIO_BUCKET
mc anonymous set download myminio/$MINIO_PUBLIC_BUCKET

mc admin accesskey create myminio/ "$MINIO_ROOT_USER" \
  --access-key "$MINIO_ACCESS_KEY" \
  --secret-key "$MINIO_ACCESS_SECRET"

echo "Uploading public-assets folder..."
if ! mc cp --recursive /scripts/public-assets/ "myminio/${MINIO_PUBLIC_BUCKET}/public-assets/"; then
  echo "Failed to upload public-assets folder" >&2
  exit 1
fi

upload_realm_config \
  "/scripts/public-assets/defaults/keycloak/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json" \
  "$KEYCLOAK_TENANT_REALM_CONFIG_S3_KEY" || exit 1
upload_realm_config \
  "/scripts/public-assets/defaults/keycloak/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559.json" \
  "$KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_S3_KEY" || exit 1

if mc stat "myminio/${MINIO_PUBLIC_BUCKET}/certs.json" > /dev/null 2>&1; then
  echo "certs.json already exists in MinIO, skipping upload..."
else
  echo "Uploading certs.json..."
  mc cp /scripts/certs.json "myminio/${MINIO_PUBLIC_BUCKET}/certs.json"
fi

exit 0

#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

mc alias set myminio "$MINIO_PRIVATE_URI" "$MINIO_ROOT_USER" "$MINIO_ROOT_PASSWORD"
mc mb -p myminio/$MINIO_PUBLIC_BUCKET
mc mb -p myminio/$MINIO_BUCKET
# Allow public object reads without revealing the bucket's object keys.
public_bucket_policy="$(mktemp)"
cat > "$public_bucket_policy" <<EOF
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Principal": {"AWS": ["*"]},
    "Action": ["s3:GetObject"],
    "Resource": ["arn:aws:s3:::${MINIO_PUBLIC_BUCKET}/*"]
  }]
}
EOF
if ! mc anonymous set-json "$public_bucket_policy" "myminio/${MINIO_PUBLIC_BUCKET}"; then
  rm -f "$public_bucket_policy"
  exit 1
fi
rm -f "$public_bucket_policy"

mc admin accesskey create myminio/ "$MINIO_ROOT_USER" \
  --access-key "$MINIO_ACCESS_KEY" \
  --secret-key "$MINIO_ACCESS_SECRET"
  
mc stat myminio/public/certs.json
if [ $? -eq 1 ]; then
  echo "Uploading certs.json..."
  mc cp /scripts/certs.json myminio/public/certs.json
else
  echo "certs.json already exists."
fi

echo "Uploading public-assets folder..."
mc cp --recursive /scripts/public-assets/ myminio/public/public-assets/

exit 0

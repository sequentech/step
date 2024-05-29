#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

mc config host add myminio $MINIO_PRIVATE_URI $MINIO_ROOT_USER $MINIO_ROOT_PASSWORD
mc mb -p myminio/$MINIO_PUBLIC_BUCKET
mc mb -p myminio/$MINIO_BUCKET
mc anonymous set download myminio/$MINIO_PUBLIC_BUCKET
mc admin user svcacct add --access-key $MINIO_ACCESS_KEY --secret-key $MINIO_ACCESS_SECRET myminio $MINIO_ROOT_USER

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

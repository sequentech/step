# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

#!/usr/bin/env bash
# Upload a compiled Wasm extension to MinIO/S3.
# Usage:  upload_extension.sh <crate-name> [version]
set -euo pipefail


export MC_HOST_myminio="${AWS_S3_PRIVATE_URI/http:\/\//http://${AWS_S3_ACCESS_KEY}:${AWS_S3_ACCESS_SECRET}@}"

CRATE="test"

if [[ -z "$CRATE" ]]; then
  echo "USAGE: $0 <crate-name> [version]" >&2
  exit 1
fi

WASM_PATH="./packages/extensions/test/rust-local-target/wasm32-unknown-unknown/debug/test.wasm"
[[ -f "$WASM_PATH" ]] || {
  echo "❌ Wasm artefact not found: $WASM_PATH" >&2
  exit 1
}

BUCKET=$AWS_S3_PUBLIC_BUCKET
OBJECT_KEY="extensions/${CRATE}.wasm"

mc cp "$WASM_PATH" "myminio/$BUCKET/$OBJECT_KEY"

echo "✅ Done  uploading – $OBJECT_KEY"

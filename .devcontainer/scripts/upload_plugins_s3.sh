#!/usr/bin/env bash
set -euo pipefail

PROTOCOL="${AWS_S3_PRIVATE_URI%%://*}"
HOSTPORT="${AWS_S3_PRIVATE_URI#*://}"
export MC_HOST_myminio="${PROTOCOL}://${AWS_S3_ACCESS_KEY}:${AWS_S3_ACCESS_SECRET}@${HOSTPORT}"

BUCKET="${AWS_S3_PUBLIC_BUCKET:?BUCKET must be set}"

# repo root
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

# Loop every subfolder in packages/extensions
for EXT_DIR in "$ROOT"/packages/plugins-manager/*; do
  [ -d "$EXT_DIR" ] || continue

  EXT_NAME="$(basename "$EXT_DIR")"
  WASM_PATH="$EXT_DIR/rust-local-target/wasm32-unknown-unknown/debug/${EXT_NAME}.wasm"

  if [ ! -f "$WASM_PATH" ]; then
    echo "Wasm artefact not found: $WASM_PATH" >&2
    continue
  fi

  EXT_PATH="plugins/${EXT_NAME}.wasm"

  echo "Uploading $EXT_NAME"
  mc cp "$WASM_PATH" "myminio/$BUCKET/$EXT_PATH"
done

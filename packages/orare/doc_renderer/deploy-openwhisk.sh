#!/usr/bin/env bash

# Build the action
cargo build --release --features openwhisk

# Create the OpenWhisk package if it doesn't exist
openwhisk-cli -v --debug package create pdf-tools || true

# Deploy the action
openwhisk-cli -v --debug action update pdf-tools/pdf-renderer \
  --kind rust:1.34 \
  --main main \
  --web true \
  --annotation provide-api-key true \
  --annotation raw-http true \
  action.yml

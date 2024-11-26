#!/bin/bash

# Build the action
cargo build --release --features openwhisk

# Create the OpenWhisk package if it doesn't exist
wsk package create pdf-tools || true

# Deploy the action
wsk action update pdf-tools/pdf-renderer \
  --kind rust:1.70 \
  --main main \
  --docker sequentech/pdf-renderer:latest \
  --web true \
  --annotation provide-api-key true \
  --annotation raw-http true \
  action.yml 
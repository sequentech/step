#!/usr/bin/env bash

# Build the action (FIXME: registry.ereslibre.net)
docker build -f ./Dockerfile -t registry.ereslibre.net/doc_renderer:latest ../..

# Create the OpenWhisk package if it doesn't exist
openwhisk-cli -v --debug package create pdf-tools || true

# Deploy the action
openwhisk-cli -v --debug action update pdf-tools/pdf-renderer \
  --web no \
  --docker registry.ereslibre.net/doc_renderer:latest

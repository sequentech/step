#!/usr/bin/env bash

SKIP_BUILD=${SKIP_BUILD:-1}
IMAGE=${IMAGE:-openwhisk/doc_renderer:latest}

if [[ "$SKIP_BUILD" != "1" ]]; then
    docker build --push -f ./Dockerfile -t $IMAGE ../..
fi

# Create the OpenWhisk package if it does not exist
openwhisk-cli package create pdf-tools || true

# Create the OpenWhisk action if it doesn't exist
openwhisk-cli action create pdf-tools/doc_renderer --web yes --docker $IMAGE || true

# Update the action
openwhisk-cli action update pdf-tools/doc_renderer --web yes --docker $IMAGE

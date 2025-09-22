#!/usr/bin/env bash

set -x

DOC_RENDERER_IMAGE="localhost:5000/doc_renderer:latest"

wsk property set \
  --apihost $OPENWHISK_API_HOST \
  --auth '23bc46b1-71f6-4ed5-8c54-816aa4f8c502:123zO3xZCLrMN6v2BKK1dXYFpXlPkccOFqm12CdAsMgRU4VrNZ9lyGVCGuMDGIwP' || true

until wsk list; do
    echo "Waiting for OpenWhisk to be ready"
    sleep 1
done

wsk package create pdf-tools || \
    true

wsk action create pdf-tools/doc_renderer \
    --web no \
    --docker $DOC_RENDERER_IMAGE || \
wsk action update pdf-tools/doc_renderer \
    --web no \
    --docker $DOC_RENDERER_IMAGE || \
    true

#!/usr/bin/env bash

set -x

wsk property set \
  --apihost 'http://127.0.0.2:3233' \
  --auth '23bc46b1-71f6-4ed5-8c54-816aa4f8c502:123zO3xZCLrMN6v2BKK1dXYFpXlPkccOFqm12CdAsMgRU4VrNZ9lyGVCGuMDGIwP' || true

until wsk list; do
    echo "Waiting for OpenWhisk to be ready"
    sleep 1
done

wsk package create pdf-tools || true

wsk action create pdf-tools/doc_renderer \
  --web no \
  --docker registry.ereslibre.net/doc_renderer:latest || true

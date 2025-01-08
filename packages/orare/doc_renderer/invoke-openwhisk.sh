#!/usr/bin/env bash

curl -v \
     -u $OPENWHISK_BASIC_AUTH \
     -XPOST \
     -H 'Content-Type: application/json' \
     -d '{ "html": "hello world", "pdf_options": { "landscape": true } }' \
     "$OPENWHISK_ENDPOINT/pdf-tools/doc_renderer?blocking=true&result=true"

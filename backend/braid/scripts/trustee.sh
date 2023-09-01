#!/bin/sh
set -x
bb_helper --cache-dir /tmp/cache -s $IMMUDB_URL -i $IMMUDB_INDEX_DB -b defaultboard  -u $IMMUDB_USER -p $IMMUDB_PASSWORD upsert-init-db -l debug
gen_trustee_config > trustee.toml
trustee --server-url $IMMUDB_URL --board-index $IMMUDB_INDEX_DB --trustee-config trustee.toml
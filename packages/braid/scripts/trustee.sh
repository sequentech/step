#!/bin/sh
set -x
cd /opt/braid
bb_helper --cache-dir /tmp/cache -s $IMMUDB_URL -i $IMMUDB_INDEX_DB -b defaultboard  -u $IMMUDB_USER -p $IMMUDB_PASSWORD upsert-init-db -l debug

if [ -z "${TRUSTEE_CONFIG}" ]; then
    TRUSTEE_CONFIG=/opt/braid/trustee.toml
fi
echo TRUSTEE_CONFIG=$TRUSTEE_CONFIG

# generate TRUSTEE config, if it doesn't exist
if [ ! -f $TRUSTEE_CONFIG ]
then
    echo generating config
    gen_trustee_config > $TRUSTEE_CONFIG
else
    echo config exists
fi
cat $TRUSTEE_CONFIG

trustee --server-url $IMMUDB_URL --board-index $IMMUDB_INDEX_DB --trustee-config $TRUSTEE_CONFIG
#!/bin/sh

set -x
cd /opt/braid
bb_helper --cache-dir /tmp/cache -s $IMMUDB_URL -i $IMMUDB_INDEX_DB -b defaultboard -u $IMMUDB_USER -p $IMMUDB_PASSWORD upsert-init-db -l debug

# generate pm config, if it doesn't exist
PM_CONFIG=/opt/braid/pm.toml
if [ ! -f $PM_CONFIG ]
then
    gen_trustee_config protocol-manager > $PM_CONFIG
fi

export ROCKET_ADDRESS=0.0.0.0
protocol_manager --server-url $IMMUDB_URL --config $PM_CONFIG

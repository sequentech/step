#!/bin/bash
set -ex

psql \
    -v ON_ERROR_STOP=1 \
    --username "$POSTGRES_USER" \
    --dbname "$POSTGRES_DB" \
    <<-EOSQL
    CREATE EXTENSION IF NOT EXISTS pgaudit;
    CREATE EXTENSION IF NOT EXISTS pgcrypto;
EOSQL

{ echo "host replication $POSTGRES_USER 0.0.0.0/0 trust"; } >> "$PGDATA/pg_hba.conf"

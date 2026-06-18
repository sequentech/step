#!/bin/bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex

psql \
    -v ON_ERROR_STOP=1 \
    --username "$POSTGRES_USER" \
    --dbname "$POSTGRES_DB" \
    -f /docker-entrypoint-initdb.d/init.sql

{ echo "host replication $POSTGRES_USER 0.0.0.0/0 trust"; } >> "$PGDATA/pg_hba.conf"

#!/bin/bash
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -euo pipefail

: "${PGHOST:?PGHOST must be set}"
: "${PGPORT:?PGPORT must be set}"
: "${PGUSER:?PGUSER must be set}"
: "${PGPASSWORD:?PGPASSWORD must be set}"
: "${B4_PG_DATABASE:?B4_PG_DATABASE must be set}"

schema_path="${B4_PG_SCHEMA_PATH:-/postgresql-b4/schema.sql}"

if [[ ! "$B4_PG_DATABASE" =~ ^[A-Za-z0-9_]+$ ]]; then
    echo "Invalid B4_PG_DATABASE: $B4_PG_DATABASE" >&2
    exit 1
fi

if ! psql -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname = '$B4_PG_DATABASE'" | grep -qx 1; then
    createdb "$B4_PG_DATABASE"
fi

psql -v ON_ERROR_STOP=1 -d "$B4_PG_DATABASE" -f "$schema_path"

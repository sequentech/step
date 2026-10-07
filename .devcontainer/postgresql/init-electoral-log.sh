#!/bin/sh
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -eu

# Only the PostgreSQL service hosting the electoral log sets these variables.
if [ -z "${ELECTORAL_LOG_PG_DATABASE:-}" ]; then exit 0; fi
: "${ELECTORAL_LOG_PG_USER:?must be set}"
: "${ELECTORAL_LOG_PG_PASSWORD:?must be set}"

psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER:-postgres}" --dbname postgres \
    -v log_user="$ELECTORAL_LOG_PG_USER" -v log_password="$ELECTORAL_LOG_PG_PASSWORD" \
    -v log_database="$ELECTORAL_LOG_PG_DATABASE" <<'SQL'
SELECT format('CREATE ROLE %I LOGIN PASSWORD %L', :'log_user', :'log_password')
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = :'log_user') \gexec
SELECT format('CREATE DATABASE %I OWNER %I', :'log_database', :'log_user')
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = :'log_database') \gexec
SQL

PGPASSWORD="$ELECTORAL_LOG_PG_PASSWORD" psql -v ON_ERROR_STOP=1 \
    --username "$ELECTORAL_LOG_PG_USER" --dbname "$ELECTORAL_LOG_PG_DATABASE" \
    --file /electoral-log-schema.sql

# The role that creates tenant databases: it may create databases and act as the
# application role, which owns them.
if [ -n "${ELECTORAL_LOG_PG_PROVISIONING_USER:-}" ]; then
    : "${ELECTORAL_LOG_PG_PROVISIONING_PASSWORD:?must be set}"
    psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER:-postgres}" --dbname postgres \
        -v log_user="$ELECTORAL_LOG_PG_USER" \
        -v provisioning_user="$ELECTORAL_LOG_PG_PROVISIONING_USER" \
        -v provisioning_password="$ELECTORAL_LOG_PG_PROVISIONING_PASSWORD" <<'SQL'
SELECT format('CREATE ROLE %I LOGIN CREATEDB PASSWORD %L', :'provisioning_user', :'provisioning_password')
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = :'provisioning_user') \gexec
SELECT format('GRANT %I TO %I', :'log_user', :'provisioning_user') \gexec
SQL
fi

# The role that administrators' console queries run as. It only connects and reads:
# provisioning lets it read each tenant database, and it gets nothing on the shared
# database, which holds every tenant's boards.
if [ -n "${ELECTORAL_LOG_PG_READER_USER:-}" ]; then
    : "${ELECTORAL_LOG_PG_READER_PASSWORD:?must be set}"
    psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER:-postgres}" --dbname postgres \
        -v reader_user="$ELECTORAL_LOG_PG_READER_USER" \
        -v reader_password="$ELECTORAL_LOG_PG_READER_PASSWORD" <<'SQL'
SELECT format('CREATE ROLE %I LOGIN PASSWORD %L', :'reader_user', :'reader_password')
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = :'reader_user') \gexec
SELECT format('ALTER ROLE %I SET default_transaction_read_only = on', :'reader_user') \gexec
SELECT format('ALTER ROLE %I SET statement_timeout = %L', :'reader_user', '30s') \gexec
SQL
fi

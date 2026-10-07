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

# The role that the super-admin tenant's console queries run as. It only connects and
# reads the electoral-log database, which holds every tenant's data.
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
    PGPASSWORD="$ELECTORAL_LOG_PG_PASSWORD" psql -v ON_ERROR_STOP=1 \
        --username "$ELECTORAL_LOG_PG_USER" --dbname "$ELECTORAL_LOG_PG_DATABASE" \
        -v reader_user="$ELECTORAL_LOG_PG_READER_USER" <<'SQL'
GRANT USAGE ON SCHEMA public TO :"reader_user";
GRANT SELECT ON ALL TABLES IN SCHEMA public TO :"reader_user";
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO :"reader_user";
SQL
fi

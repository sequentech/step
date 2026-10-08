#!/bin/sh
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -eu

# Only the PostgreSQL service hosting the electoral log sets these variables.
if [ -z "${ELECTORAL_LOG_PG_DATABASE:-}" ]; then exit 0; fi
: "${ELECTORAL_LOG_PG_USER:?must be set}"
: "${ELECTORAL_LOG_PG_PASSWORD:?must be set}"

# The application's role creates a database per election event, so it may create
# databases. The base database holds the catalog of the event databases.
psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER:-postgres}" --dbname postgres \
    -v log_user="$ELECTORAL_LOG_PG_USER" -v log_password="$ELECTORAL_LOG_PG_PASSWORD" \
    -v log_database="$ELECTORAL_LOG_PG_DATABASE" <<'SQL'
SELECT format('CREATE ROLE %I LOGIN CREATEDB PASSWORD %L', :'log_user', :'log_password')
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = :'log_user') \gexec
SELECT format('ALTER ROLE %I CREATEDB', :'log_user') \gexec
SELECT format('CREATE DATABASE %I OWNER %I', :'log_database', :'log_user')
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = :'log_database') \gexec
SQL

PGPASSWORD="$ELECTORAL_LOG_PG_PASSWORD" psql -v ON_ERROR_STOP=1 \
    --username "$ELECTORAL_LOG_PG_USER" --dbname "$ELECTORAL_LOG_PG_DATABASE" \
    --file /electoral-log-catalog.sql

# The role that the super-admin tenant's console queries run as. Each election event's
# database lets it connect and read when the application creates the database.
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

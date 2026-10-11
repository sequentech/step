#!/bin/sh
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -eu

# Only the PostgreSQL service hosting the electoral log sets these variables.
if [ -z "${ELECTORAL_LOG_PG_DATABASE:-}" ]; then exit 0; fi
: "${ELECTORAL_LOG_PG_USER:?must be set}"
: "${ELECTORAL_LOG_PG_PASSWORD:?must be set}"

# The application's role owns the base database, which holds the catalog of the event
# databases, and each election event's database.
psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER:-postgres}" --dbname postgres \
    -v log_user="$ELECTORAL_LOG_PG_USER" -v log_password="$ELECTORAL_LOG_PG_PASSWORD" \
    -v log_database="$ELECTORAL_LOG_PG_DATABASE" <<'SQL'
SELECT format('CREATE ROLE %I LOGIN PASSWORD %L', :'log_user', :'log_password')
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = :'log_user') \gexec
SELECT format('CREATE DATABASE %I OWNER %I', :'log_database', :'log_user')
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = :'log_database') \gexec
SQL

# The role that creates and drops the event databases: it may create databases and act
# as the application role, which owns them, so the application role may not create
# any. Without it, the application role creates them itself.
if [ -n "${ELECTORAL_LOG_PG_PROVISIONING_USER:-}" ]; then
    : "${ELECTORAL_LOG_PG_PROVISIONING_PASSWORD:?must be set}"
    psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER:-postgres}" --dbname postgres \
        -v log_user="$ELECTORAL_LOG_PG_USER" \
        -v provisioning_user="$ELECTORAL_LOG_PG_PROVISIONING_USER" \
        -v provisioning_password="$ELECTORAL_LOG_PG_PROVISIONING_PASSWORD" <<'SQL'
SELECT format('CREATE ROLE %I LOGIN CREATEDB PASSWORD %L', :'provisioning_user', :'provisioning_password')
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = :'provisioning_user') \gexec
SELECT format('ALTER ROLE %I CREATEDB', :'provisioning_user') \gexec
SELECT format('GRANT %I TO %I', :'log_user', :'provisioning_user') \gexec
SELECT format('ALTER ROLE %I NOCREATEDB', :'log_user') \gexec
SQL
else
    psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER:-postgres}" --dbname postgres \
        -v log_user="$ELECTORAL_LOG_PG_USER" <<'SQL'
SELECT format('ALTER ROLE %I CREATEDB', :'log_user') \gexec
SQL
fi

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

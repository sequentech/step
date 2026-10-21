#!/bin/sh
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
set -eu

# Creates the environment's task-queue database and the roles of its components, as
# provisioning does in a deployment. The owner then installs PGMQ and the queues with
# Windmill's `setup-queue-database`.
# Only the PostgreSQL service hosting the task queues sets these variables.
if [ -z "${QUEUE_DB__DBNAME:-}" ]; then exit 0; fi
: "${QUEUE_DB_OWNER_USER:?must be set}"
: "${QUEUE_DB_OWNER_PASSWORD:?must be set}"
: "${QUEUE_DB_WORKER_USER:?must be set}"
: "${QUEUE_DB_WORKER_PASSWORD:?must be set}"
: "${QUEUE_DB_PRODUCER_USER:?must be set}"
: "${QUEUE_DB_PRODUCER_PASSWORD:?must be set}"
: "${QUEUE_DB_READER_USER:?must be set}"
: "${QUEUE_DB_READER_PASSWORD:?must be set}"

psql -v ON_ERROR_STOP=1 --username "${POSTGRES_USER:-postgres}" --dbname postgres \
    -v database="$QUEUE_DB__DBNAME" \
    -v owner="$QUEUE_DB_OWNER_USER" -v owner_password="$QUEUE_DB_OWNER_PASSWORD" \
    -v worker="$QUEUE_DB_WORKER_USER" -v worker_password="$QUEUE_DB_WORKER_PASSWORD" \
    -v producer="$QUEUE_DB_PRODUCER_USER" -v producer_password="$QUEUE_DB_PRODUCER_PASSWORD" \
    -v reader="$QUEUE_DB_READER_USER" -v reader_password="$QUEUE_DB_READER_PASSWORD" <<'SQL'
SELECT format('CREATE ROLE %I LOGIN PASSWORD %L', role_name, role_password)
FROM (VALUES (:'owner', :'owner_password'), (:'worker', :'worker_password'),
             (:'producer', :'producer_password'), (:'reader', :'reader_password'))
    AS roles (role_name, role_password)
WHERE NOT EXISTS (SELECT FROM pg_roles WHERE rolname = role_name) \gexec
SELECT format('ALTER ROLE %I SET default_transaction_read_only = on', :'reader') \gexec
SELECT format('ALTER ROLE %I SET statement_timeout = %L', :'reader', '30s') \gexec
SELECT format('CREATE DATABASE %I OWNER %I', :'database', :'owner')
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = :'database') \gexec
SQL

#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
# Test the worker's verbatim ACL checks in a fresh, socket-only PG cluster.
set -euo pipefail
PGBIN=${1:?provide PostgreSQL 17 bin directory}
SCRIPTS=$(cd "$(dirname "$0")" && pwd)
SQL="$SCRIPTS/../../packages/windmill/src/operational_demo"
TEMP=$(mktemp -d /tmp/demo-worker-acl.XXXXXXXX)
trap '"$PGBIN/pg_ctl" -D "$TEMP/pgdata" -m immediate stop >/dev/null 2>&1 || true; rm -rf "$TEMP"' EXIT
"$PGBIN/initdb" -U demo_owner -D "$TEMP/pgdata" --auth-local=trust --auth-host=reject --no-locale >"$TEMP/init.log"
"$PGBIN/pg_ctl" -D "$TEMP/pgdata" -l "$TEMP/server.log" -o "-k $TEMP -p 55861 -c listen_addresses=''" -w start >/dev/null
psql() { env -i PATH=/usr/bin:/bin "$PGBIN/psql" -XAtv ON_ERROR_STOP=1 -h "$TEMP" -p 55861 "$@"; }
psql -U demo_owner -d postgres <<'SQL'
CREATE DATABASE operational_demo_14990;
CREATE DATABASE operational_demo_queues_14990;
CREATE ROLE os14990_windmill LOGIN;
CREATE ROLE os14990_queue LOGIN;
CREATE ROLE sequent_os_approve NOLOGIN;
CREATE ROLE sequent_os_initialize_device NOLOGIN;
GRANT sequent_os_approve TO os14990_windmill;
SQL
psql -U demo_owner -d operational_demo_14990 <<'SQL'
CREATE SCHEMA sequent_backend;
CREATE TABLE sequent_backend.operational_signing_intent(x int);
GRANT USAGE ON SCHEMA sequent_backend TO sequent_os_approve;
SQL
psql -U demo_owner -d operational_demo_queues_14990 <<'SQL'
CREATE SCHEMA step_queue;
CREATE TABLE step_queue.installation(environment text);
INSERT INTO step_queue.installation VALUES ('operational_demo_14990');
GRANT USAGE ON SCHEMA step_queue TO os14990_queue;
GRANT SELECT ON step_queue.installation TO os14990_queue;
CREATE SCHEMA pgmq;
CREATE TABLE pgmq.meta(queue_name text PRIMARY KEY);
INSERT INTO pgmq.meta VALUES ('short_queue'),('electoral_log_beat_queue');
GRANT USAGE ON SCHEMA pgmq TO os14990_queue;
GRANT SELECT ON pgmq.meta TO os14990_queue;
SQL
app() { psql -U "$1" -d operational_demo_14990 -f "$SQL/app-ready.sql"; }
queues() { psql -U os14990_queue -d operational_demo_queues_14990 -f "$SQL/queues-ready.sql"; }
test "$(app os14990_windmill)" = t
test "$(app demo_owner)" = f
test "$(queues)" = t
psql -U demo_owner -d operational_demo_queues_14990 -c "UPDATE step_queue.installation SET environment='other';"
test "$(queues)" = f
psql -U demo_owner -d operational_demo_queues_14990 -c "UPDATE step_queue.installation SET environment='operational_demo_14990';"
psql -U demo_owner -d operational_demo_14990 -c 'GRANT UPDATE(x) ON sequent_backend.operational_signing_intent TO os14990_windmill;'
test "$(app os14990_windmill)" = f
psql -U demo_owner -d operational_demo_14990 -c 'REVOKE UPDATE(x) ON sequent_backend.operational_signing_intent FROM os14990_windmill; GRANT sequent_os_initialize_device TO os14990_windmill;'
test "$(app os14990_windmill)" = f
psql -U demo_owner -d operational_demo_queues_14990 -c "DELETE FROM pgmq.meta WHERE queue_name='electoral_log_beat_queue';"
test "$(queues)" = f
echo 'PASS worker/queue readiness, privileged and raw-write users refused, queue loss refused'

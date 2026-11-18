-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Prepares an environment's task-queue database. The database owner runs it in one
-- transaction, after PGMQ is installed, with these transaction-local settings:
--   step.queue.environment    the environment (ENV_SLUG) the database belongs to
--   step.queue.pgmq_version   the PGMQ version of the release
--   step.queue.queues         the queue names, separated by commas
--   step.queue.worker_role    the role of the workers and the scheduler, or empty
--   step.queue.producer_role  the role of the services that only enqueue tasks, or empty
--   step.queue.reader_role    the role of read-only queue inspection, or empty
-- It is idempotent: running it again creates the queues added since and grants on them.

CREATE SCHEMA IF NOT EXISTS step_queue;

CREATE TABLE IF NOT EXISTS step_queue.installation (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    environment text NOT NULL CHECK (environment <> ''),
    pgmq_version text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

DO $$
DECLARE
    expected_environment text := current_setting('step.queue.environment');
    expected_version text := current_setting('step.queue.pgmq_version');
    installed step_queue.installation;
BEGIN
    SELECT * INTO installed FROM step_queue.installation;
    IF NOT FOUND THEN
        INSERT INTO step_queue.installation (environment, pgmq_version)
        VALUES (expected_environment, expected_version);
    ELSIF installed.environment <> expected_environment THEN
        RAISE EXCEPTION 'This task-queue database belongs to environment %, not %',
            installed.environment, expected_environment;
    ELSIF installed.pgmq_version <> expected_version THEN
        RAISE EXCEPTION 'This task-queue database has PGMQ %, but this release uses PGMQ %',
            installed.pgmq_version, expected_version;
    END IF;
END
$$;

DO $$
DECLARE
    queue text;
BEGIN
    FOREACH queue IN ARRAY string_to_array(current_setting('step.queue.queues'), ',') LOOP
        PERFORM pgmq.create(queue);
    END LOOP;
END
$$;

DO $$
DECLARE
    worker text := current_setting('step.queue.worker_role');
    producer text := current_setting('step.queue.producer_role');
    reader text := current_setting('step.queue.reader_role');
    grantee text;
    queue text;
BEGIN
    IF worker = '' AND producer = '' AND reader = '' THEN
        RETURN;
    END IF;
    -- Other environments' roles on the same server cannot connect.
    EXECUTE format('REVOKE ALL ON DATABASE %I FROM PUBLIC', current_database());
    FOREACH grantee IN ARRAY ARRAY[worker, producer, reader] LOOP
        IF grantee <> '' THEN
            EXECUTE format('GRANT CONNECT ON DATABASE %I TO %I', current_database(), grantee);
            EXECUTE format('GRANT USAGE ON SCHEMA pgmq, step_queue TO %I', grantee);
            EXECUTE format('GRANT SELECT ON step_queue.installation, pgmq.meta TO %I', grantee);
        END IF;
    END LOOP;
    FOREACH queue IN ARRAY string_to_array(current_setting('step.queue.queues'), ',') LOOP
        IF worker <> '' THEN
            EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON pgmq.%I TO %I',
                'q_' || queue, worker);
            EXECUTE format('GRANT SELECT, INSERT, DELETE ON pgmq.%I TO %I', 'a_' || queue, worker);
            EXECUTE format('GRANT SELECT ON SEQUENCE pgmq.%I TO %I',
                'q_' || queue || '_msg_id_seq', worker);
        END IF;
        IF producer <> '' THEN
            -- pgmq.send returns the new message's ID.
            EXECUTE format('GRANT SELECT (msg_id), INSERT ON pgmq.%I TO %I', 'q_' || queue, producer);
        END IF;
        IF reader <> '' THEN
            EXECUTE format('GRANT SELECT ON pgmq.%I, pgmq.%I TO %I',
                'q_' || queue, 'a_' || queue, reader);
            EXECUTE format('GRANT SELECT ON SEQUENCE pgmq.%I TO %I',
                'q_' || queue || '_msg_id_seq', reader);
        END IF;
    END LOOP;
END
$$;

-- The electoral log's entries that wait for their board's signer (Windmill's
-- `services::electoral_log_deferral`): one row per delivery ID, in a table only
-- the workers write. An event waiting on the event queue has its row (`statement`
-- NULL); an unsigned statement a worker queued has its typed statement here, and the
-- event queue only carries a reference to it. Producers (Harvest, Keycloak) can
-- neither add nor change rows: writing to the queues gives no way to have a board
-- sign a statement, nor to hold a log's direct posts.
CREATE TABLE IF NOT EXISTS step_queue.electoral_log_waiting (
    delivery_id text PRIMARY KEY,
    scope text NOT NULL,
    since_ms bigint NOT NULL,
    ord bigint NOT NULL,
    due_at timestamptz NOT NULL,
    statement jsonb,
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS electoral_log_waiting_scope
    ON step_queue.electoral_log_waiting (scope);
-- The message that is the row's copy on the event queue, if one was sent: a
-- put-back checks that exact message, not producer-writable headers.
ALTER TABLE step_queue.electoral_log_waiting ADD COLUMN IF NOT EXISTS copy_msg_id bigint;
-- A system-signed pending statement's MAC, copied from the action database's
-- outbox: the batch task checks it again before signing.
ALTER TABLE step_queue.electoral_log_waiting ADD COLUMN IF NOT EXISTS mac bytea;

-- A waiting event's copy on a queue is looked up by its delivery header.
DO $$
DECLARE
    queue text;
BEGIN
    FOREACH queue IN ARRAY string_to_array(current_setting('step.queue.queues'), ',') LOOP
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS %I ON pgmq.%I ((headers->>%L)) WHERE headers->>%L IS NOT NULL',
            'q_' || queue || '_deferred_delivery', 'q_' || queue,
            'x-electoral-log-deferred-delivery', 'x-electoral-log-deferred-scope');
    END LOOP;
END
$$;

DO $$
DECLARE
    worker text := current_setting('step.queue.worker_role');
    reader text := current_setting('step.queue.reader_role');
BEGIN
    IF worker <> '' THEN
        EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON step_queue.electoral_log_waiting TO %I',
            worker);
    END IF;
    IF reader <> '' THEN
        EXECUTE format('GRANT SELECT ON step_queue.electoral_log_waiting TO %I', reader);
    END IF;
END
$$;

-- Harvest (the producer role) posts scoped entries itself, so it must see whether
-- a scope has entries waiting (the direct-post gate): it may read the scope column
-- only, never a statement, and may not write the table.
DO $$
DECLARE
    producer text := current_setting('step.queue.producer_role');
BEGIN
    IF producer <> '' THEN
        EXECUTE format('GRANT SELECT (scope) ON step_queue.electoral_log_waiting TO %I', producer);
    END IF;
END
$$;

-- No function lets the producer role queue a pending statement: it is shared by
-- Harvest and Keycloak, so it would let Keycloak have entries signed. (An earlier
-- revision had one; it is dropped.)
DROP FUNCTION IF EXISTS step_queue.queue_pending_statement(text, jsonb, jsonb, integer);

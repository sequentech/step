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

-- The one way the producer role (Harvest; Keycloak shares the role) may queue an
-- unsigned electoral-log statement: this function, which runs as its owner and
-- accepts only the kinds the producer role is allowed, checked here, nothing
-- broader. It is not a grant on the waiting table. What it accepts is still
-- checked again by the batch task before anything is signed.
-- Allowed now: a ballot publication's failure (Harvest's publish route).
CREATE OR REPLACE FUNCTION step_queue.queue_pending_statement(
    p_delivery_id text,
    p_statement jsonb,
    p_reference jsonb,
    p_delay_secs integer
) RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = pg_catalog, pg_temp
AS $$
DECLARE
    v_scope text;
    v_occurred bigint;
BEGIN
    IF jsonb_typeof(p_statement) <> 'object'
        OR (SELECT array_agg(k ORDER BY k) FROM jsonb_object_keys(p_statement) AS k)
            <> ARRAY['election_event_id', 'occurred_at', 'origin', 'statement', 'tenant_id', 'version']
        OR p_statement->'version' <> '1'::jsonb
        OR p_statement->>'origin' <> 'ballot-publication'
        OR jsonb_typeof(p_statement->'statement') <> 'object'
        OR p_statement->'statement'->>'kind' <> 'ballot-publication-failure'
        OR EXISTS (
            SELECT 1 FROM jsonb_object_keys(p_statement->'statement') AS k
            WHERE k NOT IN ('kind', 'failure', 'username'))
        OR p_delivery_id !~ '^pending:ballot-publication:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        OR p_statement->>'tenant_id' !~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        OR p_statement->>'election_event_id' !~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
        OR jsonb_typeof(p_statement->'occurred_at') <> 'number'
        OR p_delay_secs IS NULL OR p_delay_secs < 0 OR p_delay_secs > 3600
    THEN
        RAISE EXCEPTION 'not a pending statement the producer role may queue'
            USING ERRCODE = 'insufficient_privilege';
    END IF;
    v_occurred := (p_statement->>'occurred_at')::bigint;
    IF abs(v_occurred - extract(epoch FROM clock_timestamp())::bigint) > 300 THEN
        RAISE EXCEPTION 'a pending statement must be queued when it happens'
            USING ERRCODE = 'insufficient_privilege';
    END IF;
    v_scope := (p_statement->>'tenant_id') || '/' || (p_statement->>'election_event_id');
    PERFORM pg_advisory_xact_lock(hashtextextended('electoral-log-scope:' || v_scope, 0));
    INSERT INTO step_queue.electoral_log_waiting (delivery_id, scope, since_ms, ord, due_at, statement)
    VALUES (p_delivery_id, v_scope, (extract(epoch FROM clock_timestamp()) * 1000)::bigint, 0,
            clock_timestamp() + make_interval(secs => p_delay_secs), p_statement);
    PERFORM pgmq.send('electoral_log_event_queue', p_reference, p_delay_secs);
END
$$;

REVOKE ALL ON FUNCTION step_queue.queue_pending_statement(text, jsonb, jsonb, integer) FROM PUBLIC;

DO $$
DECLARE
    producer text := current_setting('step.queue.producer_role');
BEGIN
    IF producer <> '' THEN
        EXECUTE format(
            'GRANT EXECUTE ON FUNCTION step_queue.queue_pending_statement(text, jsonb, jsonb, integer) TO %I',
            producer);
    END IF;
END
$$;

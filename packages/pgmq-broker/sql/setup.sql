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

-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Run with psql in a fresh disposable database as its owner. This tests the
-- storage observer grants without installing or exercising PGMQ queues.
\set ON_ERROR_STOP on
BEGIN;
CREATE SCHEMA pgmq;
CREATE TABLE pgmq.meta(queue_name text);
CREATE ROLE audit_fixture_worker NOLOGIN;
CREATE ROLE audit_fixture_producer NOLOGIN;
CREATE ROLE audit_fixture_reader NOLOGIN;
SET LOCAL step.queue.environment='audit-fixture';
SET LOCAL step.queue.pgmq_version='fixture';
SET LOCAL step.queue.queues='';
SET LOCAL step.queue.worker_role='audit_fixture_worker';
SET LOCAL step.queue.producer_role='audit_fixture_producer';
SET LOCAL step.queue.reader_role='audit_fixture_reader';
\ir setup.sql
\ir setup.sql
SET LOCAL ROLE audit_fixture_worker;
INSERT INTO step_queue.audit_storage_health(observed_at,state,used_bytes,available_bytes,filesystem_path)
VALUES(clock_timestamp(),'available',1,9,'/audit');
UPDATE step_queue.audit_storage_health SET state='threshold-exceeded';
RESET ROLE;
DO $$ DECLARE r text; BEGIN
 FOREACH r IN ARRAY ARRAY['audit_fixture_worker','audit_fixture_producer','audit_fixture_reader'] LOOP
  IF NOT has_table_privilege(r,'step_queue.audit_storage_health','SELECT') THEN
   RAISE EXCEPTION 'Observer read unavailable for %',r;
  END IF;
  IF has_table_privilege(r,'step_queue.audit_storage_health','DELETE') OR
     has_table_privilege(r,'step_queue.audit_storage_health','TRUNCATE') THEN
   RAISE EXCEPTION 'Observer data deletable by %',r;
  END IF;
 END LOOP;
END $$;
SET LOCAL ROLE audit_fixture_producer;
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM step_queue.audit_storage_health WHERE state='threshold-exceeded') THEN
  RAISE EXCEPTION 'Producer cannot observe capacity refusal';
 END IF;
 BEGIN
  UPDATE step_queue.audit_storage_health SET state='available';
  RAISE EXCEPTION 'Producer forged a healthy observation';
 EXCEPTION WHEN insufficient_privilege THEN NULL; END;
 BEGIN
  INSERT INTO step_queue.audit_storage_health(observed_at,state,filesystem_path)
  VALUES(clock_timestamp(),'available','/forged');
  RAISE EXCEPTION 'Producer inserted a forged observation';
 EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END $$;
RESET ROLE;
SET LOCAL ROLE audit_fixture_reader;
DO $$ BEGIN
 BEGIN
  UPDATE step_queue.audit_storage_health SET state='available';
  RAISE EXCEPTION 'Reader forged a healthy observation';
 EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END $$;
RESET ROLE;
ROLLBACK;
SELECT 'PASS: observer setup is idempotent; only worker can write; producer and reader can inspect but cannot forge or delete storage state' AS result;

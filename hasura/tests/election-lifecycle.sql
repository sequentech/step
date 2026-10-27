-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Run only in a new disposable database. No production database is accepted:
-- CREATE SCHEMA below fails if the application schema already exists.
\set ON_ERROR_STOP on
CREATE SCHEMA sequent_backend;
CREATE TABLE sequent_backend.tenant(id uuid PRIMARY KEY);
CREATE TABLE sequent_backend.election_event(id uuid PRIMARY KEY,tenant_id uuid NOT NULL REFERENCES sequent_backend.tenant(id),status jsonb);
CREATE FUNCTION sequent_backend.trusted_write_allowed() RETURNS boolean LANGUAGE SQL AS $$ SELECT COALESCE(current_setting('lifecycle_test.trusted',true),'false')='true' $$;
INSERT INTO sequent_backend.tenant VALUES('11111111-1111-4111-8111-111111111111'),('22222222-2222-4222-8222-222222222222');
INSERT INTO sequent_backend.election_event VALUES('33333333-3333-4333-8333-333333333333','11111111-1111-4111-8111-111111111111','{"voting_status":"OPEN"}');
\ir ../migrations/backend-db/1793020001000_election_lifecycle/up.sql
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle WHERE phase='execution' AND safety='recoverable') THEN
        RAISE EXCEPTION 'Existing execution must migrate to recovery, never silently reopen'; END IF;
    IF EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle_access WHERE voter_allowed) THEN
        RAISE EXCEPTION 'Missing checks and leases must fail closed'; END IF;
    BEGIN
        UPDATE sequent_backend.election_lifecycle SET safety='operational';
        RAISE EXCEPTION 'A direct client update was accepted';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END $$;
INSERT INTO sequent_backend.election_event VALUES('44444444-4444-4444-8444-444444444444','11111111-1111-4111-8111-111111111111',NULL);
DO $$ BEGIN
    IF NOT EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle WHERE election_event_id='44444444-4444-4444-8444-444444444444' AND phase='preparation' AND safety='recoverable' AND generation=0) THEN
        RAISE EXCEPTION 'New events must start fail-closed'; END IF;
END $$;
SET lifecycle_test.trusted='true';
DO $$ BEGIN
    BEGIN
        UPDATE sequent_backend.election_lifecycle SET tenant_id='22222222-2222-4222-8222-222222222222';
        RAISE EXCEPTION 'Cross-tenant state accepted';
    EXCEPTION WHEN check_violation THEN NULL; END;
    BEGIN
        UPDATE sequent_backend.election_lifecycle SET phase='post_processing' WHERE phase='preparation';
        RAISE EXCEPTION 'Skipped execution and evaluation phases accepted';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END $$;
UPDATE sequent_backend.election_lifecycle SET safety='unrecoverable',generation=1 WHERE phase='execution';
DO $$ BEGIN
    BEGIN
        UPDATE sequent_backend.election_lifecycle SET safety='operational' WHERE phase='execution';
        RAISE EXCEPTION 'Unrecoverable execution resumed';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
    BEGIN
        UPDATE sequent_backend.election_lifecycle SET generation=0 WHERE phase='execution';
        RAISE EXCEPTION 'Previous test generation replayed';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
    BEGIN
        UPDATE sequent_backend.election_lifecycle SET phase='preparation' WHERE phase='execution';
        RAISE EXCEPTION 'Execution returned to preparation';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END $$;
UPDATE sequent_backend.election_lifecycle SET phase='evaluation' WHERE phase='execution';
UPDATE sequent_backend.election_lifecycle SET phase='post_processing' WHERE phase='evaluation';
DO $$ BEGIN
    BEGIN
        UPDATE sequent_backend.election_lifecycle SET phase='execution' WHERE phase='post_processing';
        RAISE EXCEPTION 'Post-processing election reopened';
    EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END $$;
SELECT 'PASS: initial failure state, unauthorized writes, tenant binding, irreversible phases and unrecoverable safety' AS result;

-- Install the actual production access predicate and exercise it with live leases.
\ir ../migrations/backend-db/1793020003000_self_tests/up.sql
UPDATE sequent_backend.election_lifecycle SET phase='execution',safety='operational'
WHERE election_event_id='44444444-4444-4444-8444-444444444444';
CREATE FUNCTION pg_temp.assert_admission(expected boolean) RETURNS void LANGUAGE plpgsql AS $$
DECLARE actual boolean;
BEGIN
 SELECT voter_allowed INTO actual FROM sequent_backend.election_lifecycle_access
 WHERE election_event_id='44444444-4444-4444-8444-444444444444';
 IF actual IS DISTINCT FROM expected THEN RAISE EXCEPTION 'Expected voter admission %, got %',expected,actual; END IF;
END $$;
SELECT pg_temp.assert_admission(false);
CREATE OR REPLACE FUNCTION sequent_backend.audit_storage_allows_voters() RETURNS boolean LANGUAGE sql VOLATILE AS $$ SELECT true $$;
INSERT INTO sequent_backend.lifecycle_service_lease(component,expires_at) VALUES('harvest',clock_timestamp()+interval '45 seconds'),('windmill',clock_timestamp()+interval '45 seconds');
SELECT pg_temp.assert_admission(false);
INSERT INTO sequent_backend.self_test_run(id,tenant_id,election_event_id,generation,trigger,finished_at,outcome,result)
VALUES('55555555-5555-4555-8555-555555555555','11111111-1111-4111-8111-111111111111','44444444-4444-4444-8444-444444444444',0,'before-start',clock_timestamp(),'passed','{}');
SELECT pg_temp.assert_admission(true);
UPDATE sequent_backend.lifecycle_service_lease SET expires_at=clock_timestamp()-interval '1 second' WHERE component='harvest';
SELECT pg_temp.assert_admission(false);
UPDATE sequent_backend.lifecycle_service_lease SET expires_at=clock_timestamp()+interval '45 seconds';
UPDATE sequent_backend.self_test_run SET finished_at=clock_timestamp()-interval '16 minutes';
SELECT pg_temp.assert_admission(false);
UPDATE sequent_backend.self_test_run SET finished_at=clock_timestamp();
SELECT pg_temp.assert_admission(true);
INSERT INTO sequent_backend.self_test_run(id,tenant_id,election_event_id,generation,trigger,finished_at,outcome,result)
VALUES('66666666-6666-4666-8666-666666666666','11111111-1111-4111-8111-111111111111','44444444-4444-4444-8444-444444444444',0,'periodic',clock_timestamp(),'recoverable-failure','{}');
SELECT pg_temp.assert_admission(false);
UPDATE sequent_backend.election_lifecycle SET generation=1 WHERE election_event_id='44444444-4444-4444-8444-444444444444';
SELECT pg_temp.assert_admission(false);
INSERT INTO sequent_backend.self_test_run(id,tenant_id,election_event_id,generation,trigger,finished_at,outcome,result)
VALUES('77777777-7777-4777-8777-777777777777','11111111-1111-4111-8111-111111111111','44444444-4444-4444-8444-444444444444',1,'before-resume',clock_timestamp(),'passed','{}');
SELECT pg_temp.assert_admission(true);
UPDATE sequent_backend.self_test_run SET outcome='unrecoverable-failure' WHERE id='66666666-6666-4666-8666-666666666666';
SELECT pg_temp.assert_admission(false);
SELECT 'PASS: actual admission view, live/stale leases, missing/stale checks, latest failures, generation binding, irreversible recorded failures' AS result;

-- The normal initialization tally is allowed before the first opening. Only an
-- actual results tally/readout makes a later "first opening" invalid.
CREATE TABLE sequent_backend.tally_session(tenant_id uuid,election_event_id uuid,tally_type text,is_execution_completed boolean);
CREATE TABLE ballot_box_ballot(election_event_id uuid);
CREATE TABLE ballot_box_received(election_event_id uuid);
CREATE FUNCTION pg_temp.initial_boxes_empty() RETURNS boolean LANGUAGE SQL AS $$
 SELECT NOT EXISTS(SELECT 1 FROM sequent_backend.tally_session WHERE tenant_id='11111111-1111-4111-8111-111111111111' AND election_event_id='44444444-4444-4444-8444-444444444444' AND tally_type='ELECTORAL_RESULTS')
 AND NOT EXISTS(SELECT 1 FROM ballot_box_ballot WHERE election_event_id='44444444-4444-4444-8444-444444444444')
 AND NOT EXISTS(SELECT 1 FROM ballot_box_received WHERE election_event_id='44444444-4444-4444-8444-444444444444')
$$;
INSERT INTO sequent_backend.tally_session VALUES('11111111-1111-4111-8111-111111111111','44444444-4444-4444-8444-444444444444','INITIALIZATION_REPORT',true);
DO $$ BEGIN
 IF NOT pg_temp.initial_boxes_empty() THEN RAISE EXCEPTION 'A successful empty initialization must permit first opening'; END IF;
 INSERT INTO ballot_box_received VALUES('44444444-4444-4444-8444-444444444444');
 IF pg_temp.initial_boxes_empty() THEN RAISE EXCEPTION 'A received ballot must block first opening'; END IF;
 DELETE FROM ballot_box_received;
 INSERT INTO ballot_box_ballot VALUES('44444444-4444-4444-8444-444444444444');
 IF pg_temp.initial_boxes_empty() THEN RAISE EXCEPTION 'A cast ballot must block first opening'; END IF;
 DELETE FROM ballot_box_ballot;
 INSERT INTO sequent_backend.tally_session VALUES('11111111-1111-4111-8111-111111111111','44444444-4444-4444-8444-444444444444','ELECTORAL_RESULTS',true);
 IF pg_temp.initial_boxes_empty() THEN RAISE EXCEPTION 'A prior results tally must block first opening even if boxes were emptied'; END IF;
END $$;
SELECT 'PASS: initialization tally permitted; received/cast ballots and actual results tally refused' AS result;

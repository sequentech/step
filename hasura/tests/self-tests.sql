-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Disposable database only: CREATE SCHEMA refuses an existing application database.
\set ON_ERROR_STOP on
CREATE SCHEMA sequent_backend;
CREATE TABLE sequent_backend.tenant(id uuid PRIMARY KEY);
CREATE TABLE sequent_backend.election_event(id uuid PRIMARY KEY,tenant_id uuid NOT NULL REFERENCES sequent_backend.tenant(id),status jsonb);
CREATE FUNCTION sequent_backend.trusted_write_allowed() RETURNS boolean LANGUAGE SQL AS $$ SELECT COALESCE(current_setting('lifecycle_test.trusted',true),'false')='true' $$;
INSERT INTO sequent_backend.tenant VALUES('11111111-1111-4111-8111-111111111111'),('22222222-2222-4222-8222-222222222222');
INSERT INTO sequent_backend.election_event VALUES('33333333-3333-4333-8333-333333333333','11111111-1111-4111-8111-111111111111','{"voting_status":"OPEN"}');
\ir ../migrations/backend-db/1793020001000_election_lifecycle/up.sql
\ir ../migrations/backend-db/1793020003000_self_tests/up.sql
CREATE OR REPLACE FUNCTION sequent_backend.audit_storage_allows_voters() RETURNS boolean LANGUAGE SQL AS $$ SELECT true $$;
SET lifecycle_test.trusted='true';
UPDATE sequent_backend.election_lifecycle SET safety='operational';
INSERT INTO sequent_backend.lifecycle_service_lease(component,expires_at) VALUES('harvest',now()+interval '1 minute'),('windmill',now()+interval '1 minute');
DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle_access WHERE voter_allowed) THEN RAISE EXCEPTION 'No evidence accepted'; END IF;
 BEGIN
 INSERT INTO sequent_backend.self_test_settings VALUES('22222222-2222-4222-8222-222222222222','33333333-3333-4333-8333-333333333333','{}');
 RAISE EXCEPTION 'Cross-tenant settings accepted';
 EXCEPTION WHEN foreign_key_violation THEN NULL; END;
END $$;
INSERT INTO sequent_backend.self_test_run VALUES('44444444-4444-4444-8444-444444444444','11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333',0,'requested',now(),now(),'passed','{}');
DO $$ BEGIN IF NOT EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle_access WHERE voter_allowed) THEN RAISE EXCEPTION 'Current pass denied'; END IF; END $$;
INSERT INTO sequent_backend.self_test_run VALUES('55555555-5555-4555-8555-555555555555','11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333',0,'requested',now(),now(),'recoverable-failure','{}');
DO $$ BEGIN IF EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle_access WHERE voter_allowed) THEN RAISE EXCEPTION 'Old pass hid new failure'; END IF; END $$;
UPDATE sequent_backend.self_test_run SET outcome='passed',finished_at=now()-interval '16 minutes';
DO $$ BEGIN IF EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle_access WHERE voter_allowed) THEN RAISE EXCEPTION 'Expired pass accepted'; END IF; END $$;
UPDATE sequent_backend.self_test_run SET outcome='passed',finished_at=now(),generation=1;
DO $$ BEGIN IF EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle_access WHERE voter_allowed) THEN RAISE EXCEPTION 'Wrong generation accepted'; END IF; END $$;
UPDATE sequent_backend.self_test_run SET outcome='unrecoverable-failure' WHERE id='44444444-4444-4444-8444-444444444444';
UPDATE sequent_backend.self_test_run SET generation=0 WHERE id='55555555-5555-4555-8555-555555555555';
DO $$ BEGIN IF EXISTS(SELECT 1 FROM sequent_backend.election_lifecycle_access WHERE voter_allowed) THEN RAISE EXCEPTION 'Unrecoverable failure hidden by later success'; END IF; END $$;
DELETE FROM sequent_backend.self_test_run;
INSERT INTO sequent_backend.self_test_run VALUES('44444444-4444-4444-8444-444444444444','11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333',0,'requested',now(),NULL,NULL,'{}');
DO $$ BEGIN
 BEGIN
 INSERT INTO sequent_backend.self_test_run VALUES('55555555-5555-4555-8555-555555555555','11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333',0,'requested',now(),NULL,NULL,'{}');
 RAISE EXCEPTION 'Overlapping queued tests accepted';
 EXCEPTION WHEN unique_violation THEN NULL; END;
END $$;
SELECT 'PASS: tenant binding, absent/stale/wrong-generation evidence, newer failure, irreversible failure, queue deduplication' AS result;

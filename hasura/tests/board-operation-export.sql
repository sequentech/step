-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Disposable PostgreSQL fixture: migration guards run against actual rows.
\set ON_ERROR_STOP on
CREATE SCHEMA sequent_backend;
CREATE TABLE sequent_backend.election_event(id uuid PRIMARY KEY,tenant_id uuid NOT NULL,presentation jsonb);
CREATE TABLE sequent_backend.signing_request(id uuid PRIMARY KEY,tenant_id uuid,election_event_id uuid,action text,status text,subject jsonb);
CREATE TABLE sequent_backend.election_lifecycle(tenant_id uuid,election_event_id uuid,phase text,generation bigint);
CREATE FUNCTION sequent_backend.trusted_write_allowed() RETURNS boolean LANGUAGE SQL AS $$ SELECT COALESCE(current_setting('sequent.trusted_write',true),'off')='on' $$;
DO $$ DECLARE tab text; BEGIN
    FOREACH tab IN ARRAY ARRAY['election','contest','candidate','area','area_contest',
        'scheduled_event','ballot_publication','ballot_style','election_result','keys_ceremony','protocol_board',
        'tally_session','tally_session_contest','tally_session_execution','tally_session_resolution',
        'tally_sheet','ballot_box_seal','results_event','results_election','results_contest',
        'results_contest_candidate','results_election_area','results_area_contest','results_area_contest_candidate'] LOOP
        EXECUTE format('CREATE TABLE sequent_backend.%I(id uuid PRIMARY KEY,tenant_id uuid,election_event_id uuid,payload jsonb)',tab);
    END LOOP;
END $$;
INSERT INTO sequent_backend.election_event VALUES('33333333-3333-4333-8333-333333333333','11111111-1111-4111-8111-111111111111','{}');
INSERT INTO sequent_backend.election_lifecycle VALUES('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','post_processing',3);
\ir ../migrations/backend-db/1793020005000_board_operations/up.sql
CREATE FUNCTION public.assert_deletion(expected boolean) RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    IF sequent_backend.authorized_execution_deletion('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333') IS DISTINCT FROM expected THEN
        RAISE EXCEPTION 'Unexpected authorization: expected %', expected;
    END IF;
END $$;
INSERT INTO sequent_backend.signing_request VALUES('44444444-4444-4444-8444-444444444444','11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','delete-execution-data','waiting','{"lifecycle_generation":3,"source_revision":0}');
INSERT INTO sequent_backend.board_operation_binding(tenant_id,election_event_id,request_id,binding_kind,binding_id) VALUES('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','44444444-4444-4444-8444-444444444444','deletion','55555555-5555-4555-8555-555555555555');
SELECT public.assert_deletion(false);
SET sequent.trusted_write='on';
SET sequent.authorized_deletion='44444444-4444-4444-8444-444444444444';
SELECT public.assert_deletion(false); -- Waiting quorum, missing export.
UPDATE sequent_backend.signing_request SET status='executed';
SELECT public.assert_deletion(false); -- Consumed quorum alone does not permit deletion.
INSERT INTO sequent_backend.execution_export(tenant_id,election_event_id,document_id,request_id,source_revision,lifecycle_generation,manifest_sha256,archive_sha256,signing_public_key) VALUES('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','66666666-6666-4666-8666-666666666666','77777777-7777-4777-8777-777777777777',0,3,repeat('a',64),repeat('b',64),'fixture public key');
SELECT public.assert_deletion(true);
SET sequent.trusted_write='off';
SELECT public.assert_deletion(false); -- A direct GraphQL transaction cannot borrow this request.
SET sequent.trusted_write='on';
UPDATE sequent_backend.election_lifecycle SET generation=4;
SELECT public.assert_deletion(false);
UPDATE sequent_backend.election_lifecycle SET generation=3,phase='evaluation';
SELECT public.assert_deletion(false);
UPDATE sequent_backend.election_lifecycle SET phase='post_processing';
SELECT public.assert_deletion(true);
DO $$ BEGIN
    IF sequent_backend.authorized_execution_deletion('22222222-2222-4222-8222-222222222222','33333333-3333-4333-8333-333333333333') THEN RAISE EXCEPTION 'Cross-tenant deletion accepted'; END IF;
END $$;
SET sequent.authorized_deletion='';
INSERT INTO sequent_backend.candidate VALUES('88888888-8888-4888-8888-888888888888','11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','{}');
SET sequent.authorized_deletion='44444444-4444-4444-8444-444444444444';
SELECT public.assert_deletion(false); -- A subsequent execution-data change invalidates the exported state.
UPDATE sequent_backend.execution_export SET source_revision=1;
SELECT public.assert_deletion(false); -- Even a new export cannot change the signed deletion revision.
UPDATE sequent_backend.signing_request SET subject='{"lifecycle_generation":3,"source_revision":1}';
SELECT public.assert_deletion(true);
DELETE FROM sequent_backend.candidate;
SELECT public.assert_deletion(true); -- Verified teardown does not invalidate itself.
SELECT 'PASS: deletion requires completed quorum, current complete export, exact tenant, phase, lifecycle generation and source revision' AS result;

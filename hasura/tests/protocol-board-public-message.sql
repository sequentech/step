-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Run with psql -v ON_ERROR_STOP=1 -f this-file.sql against a disposable empty database.
BEGIN;
CREATE SCHEMA sequent_backend;
CREATE TABLE sequent_backend.tenant (id uuid PRIMARY KEY);
CREATE TABLE sequent_backend.election_event (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL REFERENCES sequent_backend.tenant (id),
    status jsonb
);
CREATE FUNCTION sequent_backend.trusted_write_allowed() RETURNS boolean LANGUAGE sql AS
$$ SELECT COALESCE(current_setting('protocol_board_test.trusted', true), '') = 'on' $$;
CREATE TABLE sequent_backend.protocol_board (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    parent_id uuid REFERENCES sequent_backend.protocol_board (id),
    manager_message bytea NOT NULL
);
\ir ../migrations/backend-db/1793020001000_election_lifecycle/up.sql
\ir ../migrations/backend-db/1793020020002_protocol_board_public_message/up.sql
CREATE FUNCTION pg_temp.message(board_id uuid) RETURNS bytea LANGUAGE sql AS $$
    SELECT sequent_backend.protocol_board_public_manager_message(b)
    FROM sequent_backend.protocol_board b
    WHERE b.id = board_id
$$;
DO $$
DECLARE
    council uuid := '00000000-0000-4000-8000-000000000001';
    other_tenant uuid := '00000000-0000-4000-8000-000000000002';
    event uuid := '00000000-0000-4000-8000-000000000003';
    finished_event uuid := '00000000-0000-4000-8000-000000000004';
    dkg uuid := '00000000-0000-4000-8000-000000000005';
    tally uuid := '00000000-0000-4000-8000-000000000006';
    finished_dkg uuid := '00000000-0000-4000-8000-000000000007';
    finished_tally uuid := '00000000-0000-4000-8000-000000000008';
    foreign_tally uuid := '00000000-0000-4000-8000-000000000009';
    next_phase text;
BEGIN
    INSERT INTO sequent_backend.tenant VALUES (council), (other_tenant);
    INSERT INTO sequent_backend.election_event VALUES (event, council, NULL), (finished_event, council, NULL);
    INSERT INTO sequent_backend.protocol_board VALUES
        (dkg, council, event, NULL, '\x01'),
        (tally, council, event, dkg, '\x02'),
        (finished_dkg, council, finished_event, NULL, '\x03'),
        (finished_tally, council, finished_event, finished_dkg, '\x04'),
        (foreign_tally, other_tenant, finished_event, finished_dkg, '\x05');

    PERFORM set_config('protocol_board_test.trusted', 'on', true);
    FOREACH next_phase IN ARRAY ARRAY['execution', 'evaluation', 'post_processing'] LOOP
        UPDATE sequent_backend.election_lifecycle SET phase = next_phase WHERE election_event_id = finished_event;
    END LOOP;

    FOREACH next_phase IN ARRAY ARRAY['preparation', 'execution', 'evaluation'] LOOP
        IF next_phase <> 'preparation' THEN
            UPDATE sequent_backend.election_lifecycle SET phase = next_phase WHERE election_event_id = event;
        END IF;
        IF pg_temp.message(dkg) IS DISTINCT FROM '\x01'::bytea THEN
            RAISE EXCEPTION 'test failed: dkg Configuration not returned in %', next_phase;
        END IF;
        IF pg_temp.message(tally) IS NOT NULL THEN
            RAISE EXCEPTION 'test failed: tally Ballots returned in %', next_phase;
        END IF;
    END LOOP;

    IF pg_temp.message(finished_tally) IS DISTINCT FROM '\x04'::bytea
       OR pg_temp.message(finished_dkg) IS DISTINCT FROM '\x03'::bytea THEN
        RAISE EXCEPTION 'test failed: messages of a post-processing event not returned';
    END IF;
    IF pg_temp.message(foreign_tally) IS NOT NULL THEN
        RAISE EXCEPTION 'test failed: tally Ballots returned for another tenant''s lifecycle';
    END IF;

    UPDATE sequent_backend.election_lifecycle SET phase = 'post_processing' WHERE election_event_id = event;
    IF pg_temp.message(tally) IS DISTINCT FROM '\x02'::bytea
       OR pg_temp.message(dkg) IS DISTINCT FROM '\x01'::bytea THEN
        RAISE EXCEPTION 'test failed: messages not returned in post-processing';
    END IF;

    DELETE FROM sequent_backend.election_lifecycle WHERE election_event_id = event;
    IF pg_temp.message(tally) IS NOT NULL OR pg_temp.message(dkg) IS DISTINCT FROM '\x01'::bytea THEN
        RAISE EXCEPTION 'test failed: tally Ballots returned without a lifecycle row';
    END IF;

    IF (SELECT provolatile FROM pg_proc
        WHERE oid = 'sequent_backend.protocol_board_public_manager_message'::regproc) <> 's'
       OR (SELECT prorettype FROM pg_proc
           WHERE oid = 'sequent_backend.protocol_board_public_manager_message'::regproc) <> 'bytea'::regtype THEN
        RAISE EXCEPTION 'test failed: the computed field function must be stable and return bytea';
    END IF;
    RAISE NOTICE 'PASS: dkg Configuration in every phase, tally Ballots only in post-processing of the same tenant and event';
END;
$$;
\ir ../migrations/backend-db/1793020020002_protocol_board_public_message/down.sql
ROLLBACK;

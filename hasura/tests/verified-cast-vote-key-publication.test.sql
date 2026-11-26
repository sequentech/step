-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

-- Only the companion script's fresh disposable schema may be used.
CREATE ROLE synthetic_cast_publisher LOGIN;
CREATE ROLE synthetic_cast_wrong_publisher LOGIN;
CREATE ROLE synthetic_cast_importer LOGIN;
CREATE ROLE synthetic_cast_unprivileged LOGIN;
GRANT sequent_os_verified_cast_key_publisher TO
    synthetic_cast_publisher, synthetic_cast_wrong_publisher;
GRANT sequent_os_verified_cast_key_importer TO synthetic_cast_importer;
GRANT USAGE ON SCHEMA sequent_backend TO synthetic_cast_unprivileged;

INSERT INTO sequent_backend.tenant VALUES
    ('11111111-1111-4111-8111-111111111111'), ('22222222-2222-4222-8222-222222222222');
INSERT INTO sequent_backend.managed_webauthn_roster
    (tenant_id, roster_json, sha256, generation, activated_by)
VALUES ('11111111-1111-4111-8111-111111111111', '{}', repeat('a',64), 7, 'synthetic-human');

-- Existing unopened and legacy-open events cannot be retrofitted by
-- replaying a past import. Legacy voting remains unchanged.
INSERT INTO sequent_backend.election_event VALUES
    ('11111111-1111-4111-8111-111111111111','55555555-5555-4555-8555-555555555555','{}'),
    ('11111111-1111-4111-8111-111111111111','66666666-6666-4666-8666-666666666666','{"voting_status":"OPEN"}');
INSERT INTO sequent_backend.configuration_package
    (tenant_id,election_event_id,external_id,revision,manifest_sha256,manifest,signer_subject,signer_serial,signer_fingerprint,approvers)
VALUES ('11111111-1111-4111-8111-111111111111','55555555-5555-4555-8555-555555555555',
    'old',1,repeat('b',64),'{}','synthetic','1','synthetic','[]');
INSERT INTO sequent_backend.verified_cast_key_site(tenant_id, publisher_login)
VALUES ('11111111-1111-4111-8111-111111111111', 'synthetic_cast_publisher');
SET ROLE synthetic_cast_importer;
DO $$
BEGIN
    ASSERT NOT sequent_backend.verified_cast_key_authorize(
        '11111111-1111-4111-8111-111111111111','55555555-5555-4555-8555-555555555555',
        1,repeat('b',64),7,repeat('a',64));
END;
$$;
RESET ROLE;

-- Model an older importer or missing Step assertion. Even a previously
-- admitted signed package must roll back on the registered receiver.
DO $$
BEGIN
    BEGIN
        INSERT INTO sequent_backend.election_event VALUES
            ('11111111-1111-4111-8111-111111111111','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb','{}');
        INSERT INTO sequent_backend.configuration_package
            (tenant_id,election_event_id,external_id,revision,manifest_sha256,manifest,signer_subject,signer_serial,signer_fingerprint,approvers)
        VALUES ('11111111-1111-4111-8111-111111111111','bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',
            'no-receipt',1,repeat('b',64),'{}','synthetic','1','synthetic','[]');
        SET CONSTRAINTS sequent_backend.verified_cast_key_registered_import_guard IMMEDIATE;
        RAISE EXCEPTION 'The registered receiver accepted an import without a key receipt';
    EXCEPTION WHEN check_violation THEN
        ASSERT SQLERRM LIKE 'the registered receiver needs an authorized cast vote key receipt%';
    END;
    ASSERT NOT EXISTS (SELECT FROM sequent_backend.election_event WHERE id = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb');
END;
$$;

-- Both rows are in this transaction, but an archive already opened cannot
-- qualify. Roll back the rejected import just like the application.
BEGIN;
INSERT INTO sequent_backend.election_event VALUES
    ('11111111-1111-4111-8111-111111111111','77777777-7777-4777-8777-777777777777','{"voting_status":"OPEN"}');
INSERT INTO sequent_backend.configuration_package
    (tenant_id,election_event_id,external_id,revision,manifest_sha256,manifest,signer_subject,signer_serial,signer_fingerprint,approvers)
VALUES ('11111111-1111-4111-8111-111111111111','77777777-7777-4777-8777-777777777777',
    'already-open',3,repeat('c',64),'{}','synthetic','1','synthetic','[]');
SET ROLE synthetic_cast_importer;
DO $$
BEGIN
    ASSERT NOT sequent_backend.verified_cast_key_authorize(
        '11111111-1111-4111-8111-111111111111','77777777-7777-4777-8777-777777777777',
        3,repeat('c',64),7,repeat('a',64));
END;
$$;
ROLLBACK;
RESET ROLE;

-- No work may escape a rolled-back signed import.
BEGIN;
INSERT INTO sequent_backend.election_event VALUES
    ('11111111-1111-4111-8111-111111111111','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa','{}');
INSERT INTO sequent_backend.configuration_package
    (tenant_id,election_event_id,external_id,revision,manifest_sha256,manifest,signer_subject,signer_serial,signer_fingerprint,approvers)
VALUES ('11111111-1111-4111-8111-111111111111','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
    'aborted',1,repeat('a',64),'{}','synthetic','1','synthetic','[]');
SET ROLE synthetic_cast_importer;
DO $$
BEGIN
    ASSERT sequent_backend.verified_cast_key_authorize(
        '11111111-1111-4111-8111-111111111111','aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
        1,repeat('a',64),7,repeat('a',64));
END;
$$;
ROLLBACK;
RESET ROLE;

BEGIN;
INSERT INTO sequent_backend.election_event VALUES
    ('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333','{}'),
    ('11111111-1111-4111-8111-111111111111','44444444-4444-4444-8444-444444444444','{}'),
    ('22222222-2222-4222-8222-222222222222','88888888-8888-4888-8888-888888888888','{}');
INSERT INTO sequent_backend.election VALUES
    ('11111111-1111-4111-8111-111111111111','33333333-3333-4333-8333-333333333333',
     '99999999-9999-4999-8999-999999999999','{}');
INSERT INTO sequent_backend.configuration_package
    (tenant_id,election_event_id,external_id,revision,manifest_sha256,manifest,signer_subject,signer_serial,signer_fingerprint,approvers)
SELECT tenant_id,id,id::text,3,repeat('c',64),'{}','synthetic','1','synthetic','[]'
FROM sequent_backend.election_event
WHERE id IN ('33333333-3333-4333-8333-333333333333','44444444-4444-4444-8444-444444444444',
             '88888888-8888-4888-8888-888888888888');
SET ROLE synthetic_cast_importer;
DO $$
DECLARE
    t uuid := '11111111-1111-4111-8111-111111111111';
BEGIN
    ASSERT NOT sequent_backend.verified_cast_key_authorize(t,'33333333-3333-4333-8333-333333333333',3,repeat('b',64),7,repeat('a',64));
    ASSERT NOT sequent_backend.verified_cast_key_authorize(t,'33333333-3333-4333-8333-333333333333',3,repeat('c',64),6,repeat('a',64));
    ASSERT NOT sequent_backend.verified_cast_key_authorize(t,'33333333-3333-4333-8333-333333333333',3,repeat('c',64),7,repeat('b',64));
    ASSERT NOT sequent_backend.verified_cast_key_authorize('22222222-2222-4222-8222-222222222222',
        '88888888-8888-4888-8888-888888888888',3,repeat('c',64),7,repeat('a',64));
    ASSERT sequent_backend.verified_cast_key_authorize(t,'33333333-3333-4333-8333-333333333333',3,repeat('c',64),7,repeat('a',64));
    ASSERT sequent_backend.verified_cast_key_authorize(t,'33333333-3333-4333-8333-333333333333',3,repeat('c',64),7,repeat('a',64));
    ASSERT sequent_backend.verified_cast_key_authorize(t,'44444444-4444-4444-8444-444444444444',3,repeat('c',64),7,repeat('a',64));
END;
$$;
RESET ROLE;
COMMIT;

SET SESSION AUTHORIZATION synthetic_cast_unprivileged;
DO $$
BEGIN
    ASSERT NOT has_function_privilege('sequent_backend.verified_cast_key_next()', 'EXECUTE');
    ASSERT NOT has_function_privilege('sequent_backend.verified_cast_key_for_open(uuid,uuid)', 'EXECUTE');
END;
$$;
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION synthetic_cast_wrong_publisher;
DO $$
BEGIN
    BEGIN
        PERFORM sequent_backend.verified_cast_key_next();
        RAISE EXCEPTION 'An unbound login listed requests';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
END;
$$;
RESET SESSION AUTHORIZATION;

-- Even the bound login is refused while it also holds the importer role.
GRANT sequent_os_verified_cast_key_importer TO synthetic_cast_publisher;
SET SESSION AUTHORIZATION synthetic_cast_publisher;
DO $$
BEGIN
    BEGIN
        PERFORM sequent_backend.verified_cast_key_next();
        RAISE EXCEPTION 'A login with conflicting privileges could list requests';
    EXCEPTION WHEN insufficient_privilege THEN NULL;
    END;
END;
$$;
RESET SESSION AUTHORIZATION;
REVOKE sequent_os_verified_cast_key_importer FROM synthetic_cast_publisher;

SET SESSION AUTHORIZATION synthetic_cast_publisher;
DO $$
DECLARE
    r record;
    p bytea := decode('046b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c2964fe342e2fe1a7f9b8ee7eb4a7c0f9e162bce33576b315ececbb6406837bf51f5','hex');
BEGIN
    ASSERT NOT has_table_privilege('sequent_backend.verified_cast_key_job', 'UPDATE');
    ASSERT NOT has_table_privilege('sequent_backend.verified_cast_key_site', 'UPDATE');
    ASSERT NOT has_function_privilege('sequent_backend.verified_cast_key_authorize(uuid,uuid,bigint,text,bigint,text)', 'EXECUTE');
    ASSERT NOT has_function_privilege('sequent_backend.verified_cast_key_complete(uuid,uuid,text)', 'EXECUTE');
    ASSERT NOT has_function_privilege('sequent_backend.cast_vote_seal_publish(uuid,uuid,text,integer,bytea)', 'EXECUTE');
    ASSERT (SELECT count(*) FROM sequent_backend.verified_cast_key_next()) = 2;
    SELECT * INTO STRICT r FROM sequent_backend.verified_cast_key_next()
    WHERE election_event_id = '33333333-3333-4333-8333-333333333333';
    ASSERT NOT sequent_backend.verified_cast_key_stage(
        r.job_id::uuid,r.tenant_id::uuid,r.election_event_id::uuid,repeat('d',64),p);
    ASSERT NOT sequent_backend.verified_cast_key_stage(
        r.job_id::uuid,r.tenant_id::uuid,'55555555-5555-4555-8555-555555555555',r.manifest_sha256,p);
    ASSERT sequent_backend.verified_cast_key_stage(
        r.job_id::uuid,r.tenant_id::uuid,r.election_event_id::uuid,r.manifest_sha256,p);
    ASSERT sequent_backend.verified_cast_key_stage(
        r.job_id::uuid,r.tenant_id::uuid,r.election_event_id::uuid,r.manifest_sha256,p);
    ASSERT (SELECT count(*) FROM sequent_backend.verified_cast_key_next()) = 1;
END;
$$;
RESET SESSION AUTHORIZATION;

-- Staging alone is not publication. Even direct status writes can't open.
DO $$
DECLARE
    t uuid := '11111111-1111-4111-8111-111111111111';
    e uuid := '33333333-3333-4333-8333-333333333333';
BEGIN
    ASSERT (SELECT required AND key_id IS NULL FROM sequent_backend.cast_vote_seal_state(t,e));
    BEGIN
        UPDATE sequent_backend.election_event SET status = '{"voting_status":"OPEN"}'
        WHERE (tenant_id,id) = (t,e);
        RAISE EXCEPTION 'A staged event opened';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
    BEGIN
        UPDATE sequent_backend.election SET status = '{"telephone_voting_status":"OPEN"}'
        WHERE (tenant_id,election_event_id) = (t,e);
        RAISE EXCEPTION 'A staged election opened';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
    BEGIN
        UPDATE sequent_backend.election SET status = '{"kiosk_voting_status":"CLOSED"}'
        WHERE (tenant_id,election_event_id) = (t,e);
        RAISE EXCEPTION 'An early close stranded an unfinished receipt';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
END;
$$;

-- Simulate the trusted Step caller after its typed board pin. The SQL test
-- does not exercise Rust's signer or pin verification.
SET ROLE synthetic_cast_importer;
DO $$
DECLARE
    t uuid := '11111111-1111-4111-8111-111111111111';
    e uuid := '33333333-3333-4333-8333-333333333333';
    wanted text;
BEGIN
    SELECT encode(sha256(public_key_sec1),'hex') INTO STRICT wanted
    FROM sequent_backend.verified_cast_key_for_open(t,e);
    ASSERT NOT sequent_backend.verified_cast_key_complete(t,e,repeat('d',64));
    ASSERT sequent_backend.verified_cast_key_complete(t,e,wanted);
    ASSERT sequent_backend.verified_cast_key_complete(t,e,wanted);
    ASSERT (SELECT required AND fingerprint = wanted AND key_version = 1
            FROM sequent_backend.cast_vote_seal_state(t,e));
END;
$$;
RESET ROLE;

-- Closing an unused channel after the pin leaves another channel openable.
UPDATE sequent_backend.election SET status = '{"kiosk_voting_status":"CLOSED"}'
WHERE election_event_id = '33333333-3333-4333-8333-333333333333';
UPDATE sequent_backend.election_event SET status = '{"voting_status":"OPEN"}'
WHERE id = '33333333-3333-4333-8333-333333333333';
UPDATE sequent_backend.election SET status = '{"kiosk_voting_status":"CLOSED","telephone_voting_status":"OPEN"}'
WHERE election_event_id = '33333333-3333-4333-8333-333333333333';
DO $$
BEGIN
    ASSERT (SELECT status ->> 'voting_status' = 'OPEN'
            FROM sequent_backend.election_event WHERE id = '33333333-3333-4333-8333-333333333333');
    ASSERT (SELECT NOT required FROM sequent_backend.cast_vote_seal_state(
        '11111111-1111-4111-8111-111111111111','66666666-6666-4666-8666-666666666666'));
END;
$$;

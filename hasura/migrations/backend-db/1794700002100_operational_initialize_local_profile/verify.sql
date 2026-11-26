-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Manual psql test after up.sql, on a disposable synthetic DB. Use a test
-- administrator able to SET SESSION AUTHORIZATION. Supply psql variables
-- tenant, event, post, publication, newer_publication, request, actor, local_login and https_login.
-- Import/publish with Step. Use a new explicitly required tenant, softhsm-test
-- setup and two published ballots matching the Post. Soft-delete the newer one
-- and obtain a waiting canonical Initialize request for the current original.
-- Use a current enrolled official, no outstanding intent and an empty profile.
-- The ordinary broker login inherits just the 1100 orchestrator. The local
-- login additionally inherits the 2100 marker. Neither login owns objects.
\set ON_ERROR_STOP on
BEGIN ISOLATION LEVEL READ COMMITTED;
CREATE TEMP TABLE local_checks (test text, passed boolean NOT NULL CHECK (passed)) ON COMMIT DROP;
CREATE TEMP TABLE local_case ON COMMIT DROP AS
    SELECT r.tenant_id, r.election_event_id AS event_id, r.election_id AS post_id,
           :'publication'::uuid AS publication_id, r.id AS request_id, r.payload_sha256,
           :'actor'::uuid::text AS actor, gen_random_uuid() AS installation_id, gen_random_uuid() AS attempt,
           encode(sha256(convert_to(gen_random_uuid()::text, 'UTF8')), 'hex') AS nonce,
           encode(sha256(convert_to(gen_random_uuid()::text, 'UTF8')), 'hex') AS binding,
           'http://localhost:18090/realms/tenant-' || r.tenant_id::text AS issuer
    FROM sequent_backend.signing_request AS r
    WHERE (r.tenant_id, r.election_event_id, r.election_id, r.id) =
          (:'tenant'::uuid, :'event'::uuid, :'post'::uuid, :'request'::uuid)
      AND r.status = 'waiting' AND r.action = 'initialize-voting'
      AND r.canonical_payload::jsonb #> '{subject,publication_id}' = to_jsonb(:'publication'::uuid)
      AND NOT ((r.canonical_payload::jsonb->'subject') ? 'ballot_publication_id');
-- gset requires exactly one row.
SELECT issuer, actor, installation_id FROM pg_temp.local_case \gset
GRANT INSERT ON pg_temp.local_checks TO sequent_os_initialize_orchestrator, sequent_os_approve;
GRANT SELECT ON pg_temp.local_case TO sequent_os_initialize_orchestrator, sequent_os_approve, sequent_os_ledger_data_owner;

PREPARE check_prepare(text, text, text, text, boolean) AS
    INSERT INTO pg_temp.local_checks
    SELECT $1, sequent_backend.operational_initialize_prepare(
        t.tenant_id, t.event_id, t.request_id, $4, t.payload_sha256, $3, t.attempt, $2, t.nonce, t.binding
    ) IS NOT DISTINCT FROM $5 FROM pg_temp.local_case AS t;
PREPARE install_scope AS
    INSERT INTO sequent_backend.operational_initialize_local_profile
        (installation_id, tenant_id, event_id, post_id, publication_id, action, device_profile)
    SELECT installation_id, tenant_id, event_id, post_id, publication_id, 'initialize-voting', 'softhsm-test'
    FROM pg_temp.local_case;

INSERT INTO pg_temp.local_checks SELECT 'empty initial scope',
    NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_local_profile);
INSERT INTO pg_temp.local_checks SELECT 'authority join is read-only',
    pg_relation_is_updatable('sequent_backend.operational_initialize_local_authority'::regclass, true) = 0;
INSERT INTO pg_temp.local_checks SELECT role_name || ' cannot prepare',
    NOT has_function_privilege(role_name::name,
        'sequent_backend.operational_initialize_prepare(uuid,uuid,uuid,text,text,text,uuid,text,text,text)', 'EXECUTE')
FROM (VALUES ('sequent_os_initialize_local_broker'), ('sequent_os_initialize_device')) AS roles(role_name);
INSERT INTO pg_temp.local_checks
SELECT role_name || ' lacks raw access to ' || relation_name,
    NOT has_table_privilege(role_name::name, relation_name, 'SELECT, INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES, TRIGGER')
    AND NOT has_any_column_privilege(role_name::name, relation_name, 'SELECT, INSERT, UPDATE, REFERENCES')
FROM (VALUES ('public'), ('sequent_os_approve'), ('sequent_os_initialize_orchestrator'), ('sequent_os_initialize_local_broker'),
             ('sequent_os_initialize_device'), ('sequent_os_initialize_ca'), ('sequent_os_setup_authority'),
             ('sequent_os_setup_device'), ('sequent_os_device')) AS roles(role_name)
CROSS JOIN (VALUES ('sequent_backend.operational_initialize_local_profile'),
                   ('sequent_backend.operational_initialize_local_authority')) AS raw(relation_name);
INSERT INTO pg_temp.local_checks SELECT role_name || ' cannot read the installed scope',
    NOT has_function_privilege(role_name::name,
        'sequent_backend.operational_signing_local_initialize_installation_id(uuid,uuid,uuid,uuid,text)', 'EXECUTE')
FROM (VALUES ('public'), ('sequent_os_device'), ('sequent_os_setup_device'), ('sequent_os_setup_authority'),
             ('sequent_os_initialize_ca'), ('sequent_os_initialize_orchestrator')) AS roles(role_name);
INSERT INTO pg_temp.local_checks SELECT 'Initialize device and local broker have separate memberships',
    NOT pg_has_role('sequent_os_initialize_device', 'sequent_os_initialize_local_broker', 'MEMBER')
    AND NOT pg_has_role(:'local_login', 'sequent_os_initialize_device', 'MEMBER');
SET LOCAL SESSION AUTHORIZATION :"local_login";
EXECUTE check_prepare('no installed scope', :'issuer', 'softhsm-test', :'actor', false);
RESET SESSION AUTHORIZATION;

-- A real promotion in a savepoint. The AFTER INSERT validation must undo
-- the candidate row even when the operator catches its error and continues.
SAVEPOINT promotion;
UPDATE sequent_backend.operational_signing_policy SET mode = 'hsm-required' WHERE singleton;
SET LOCAL ROLE sequent_os_ledger_data_owner;
DO $$
BEGIN
    BEGIN
        INSERT INTO sequent_backend.operational_initialize_local_profile
            (installation_id, tenant_id, event_id, post_id, publication_id, action, device_profile)
        SELECT installation_id, tenant_id, event_id, post_id, publication_id, 'initialize-voting', 'softhsm-test'
        FROM pg_temp.local_case;
        RAISE EXCEPTION 'installed a local scope under deployment-wide promotion';
    EXCEPTION WHEN check_violation THEN NULL;
    END;
    IF EXISTS (SELECT FROM sequent_backend.operational_initialize_local_profile) THEN
        RAISE EXCEPTION 'refused installation left a row';
    END IF;
END;
$$;
ROLLBACK TO promotion;
SET LOCAL ROLE sequent_os_ledger_data_owner;
EXECUTE install_scope;
RESET ROLE;

SET LOCAL ROLE sequent_os_approve;
INSERT INTO pg_temp.local_checks
SELECT 'exact installation', sequent_backend.operational_signing_local_initialize_installation_id(
    tenant_id, event_id, post_id, publication_id, 'softhsm-test') = installation_id FROM pg_temp.local_case;
INSERT INTO pg_temp.local_checks
SELECT 'wrong ' || bad.field, sequent_backend.operational_signing_local_initialize_installation_id(
    bad.tenant, bad.event, bad.post, bad.publication, bad.profile) IS NULL
FROM pg_temp.local_case AS t CROSS JOIN LATERAL (VALUES
    ('tenant', gen_random_uuid(), t.event_id, t.post_id, t.publication_id, 'softhsm-test'),
    ('event', t.tenant_id, gen_random_uuid(), t.post_id, t.publication_id, 'softhsm-test'),
    ('Post', t.tenant_id, t.event_id, gen_random_uuid(), t.publication_id, 'softhsm-test'),
    ('publication', t.tenant_id, t.event_id, t.post_id, gen_random_uuid(), 'softhsm-test'),
    ('profile', t.tenant_id, t.event_id, t.post_id, t.publication_id, 'yubihsm')
) AS bad(field, tenant, event, post, publication, profile);
RESET ROLE;

SET LOCAL ROLE sequent_os_initialize_device;
SELECT sequent_backend.operational_signing_local_initialize_installation_id(
           :'tenant'::uuid, :'event'::uuid, :'post'::uuid, :'publication'::uuid, 'softhsm-test'::text
       ) IS NOT DISTINCT FROM :'installation_id'::uuid AS device_match,
       sequent_backend.operational_signing_local_initialize_installation_id(
           :'tenant'::uuid, :'event'::uuid, :'post'::uuid, :'newer_publication'::uuid, 'softhsm-test'::text
       ) IS NULL AS device_miss \gset
RESET ROLE;
INSERT INTO pg_temp.local_checks VALUES ('device reads the installed scope', :'device_match'::boolean),
                                        ('device refuses another publication', :'device_miss'::boolean);

SAVEPOINT promoted_installation;
UPDATE sequent_backend.operational_signing_policy SET mode = 'hsm-required' WHERE singleton;
SET LOCAL SESSION AUTHORIZATION :"local_login";
INSERT INTO pg_temp.local_checks SELECT 'lookup after promotion',
    sequent_backend.operational_signing_local_initialize_installation_id(
        tenant_id, event_id, post_id, publication_id, 'softhsm-test') IS NULL FROM pg_temp.local_case;
EXECUTE check_prepare('prepare after promotion', :'issuer', 'softhsm-test', :'actor', false);
ROLLBACK TO promoted_installation;

SET LOCAL SESSION AUTHORIZATION :"https_login";
EXECUTE check_prepare('ordinary broker refuses local HTTP', :'issuer', 'softhsm-test', :'actor', false);
SAVEPOINT ordinary;
EXECUTE check_prepare('original HTTPS positive control', 'https://issuer.invalid', 'softhsm-test', :'actor', true);
ROLLBACK TO ordinary;
RESET SESSION AUTHORIZATION;

SET LOCAL SESSION AUTHORIZATION :"local_login";
EXECUTE check_prepare('local broker refuses an HTTPS substitute', 'https://issuer.invalid', 'softhsm-test', :'actor', false);
EXECUTE check_prepare('other loopback', replace(:'issuer','localhost','127.0.0.1'), 'softhsm-test', :'actor', false);
EXECUTE check_prepare('trailing slash', :'issuer' || '/', 'softhsm-test', :'actor', false);
EXECUTE check_prepare('other realm', 'http://localhost:18090/realms/tenant-' || gen_random_uuid()::text, 'softhsm-test', :'actor', false);
EXECUTE check_prepare('wrong device', :'issuer', 'yubihsm', :'actor', false);
EXECUTE check_prepare('other official', :'issuer', 'softhsm-test', gen_random_uuid()::text, false);
RESET SESSION AUTHORIZATION;

SAVEPOINT changed_scope;
-- Keep the original canonical bytes/hash and change only the unsigned copy.
UPDATE sequent_backend.signing_request AS r SET subject = r.subject - 'publication_id'
FROM pg_temp.local_case AS t WHERE r.id = t.request_id;
SET LOCAL SESSION AUTHORIZATION :"local_login";
EXECUTE check_prepare('split signed subject', :'issuer', 'softhsm-test', :'actor', false);
ROLLBACK TO changed_scope;

-- Change the signed bytes, synchronize the stored subject and submit the new
-- digest. The installed scope, request Post and caller stay the same.
PREPARE change_signed_text(text, text) AS
WITH changed AS (
    SELECT r.id, replace(r.canonical_payload, $1, $2) AS payload
    FROM sequent_backend.signing_request AS r JOIN pg_temp.local_case AS t ON r.id = t.request_id
), updated AS (
    UPDATE sequent_backend.signing_request AS r
    SET canonical_payload = c.payload, subject = c.payload::jsonb->'subject',
        payload_sha256 = encode(sha256(convert_to(c.payload,'UTF8')), 'hex')
    FROM changed AS c WHERE r.id = c.id RETURNING r.id, r.payload_sha256
)
UPDATE pg_temp.local_case AS t SET payload_sha256 = u.payload_sha256 FROM updated AS u WHERE t.request_id = u.id;
SAVEPOINT wrong_alias;
EXECUTE change_signed_text('"publication_id":', '"ballot_publication_id":');
SET LOCAL SESSION AUTHORIZATION :"local_login";
INSERT INTO pg_temp.local_checks SELECT 'wrong alias passes original context with the installed UUID', EXISTS (
    SELECT FROM pg_temp.local_case AS t CROSS JOIN LATERAL sequent_backend.operational_initialize_context(
        t.tenant_id,t.event_id,t.request_id,t.actor,t.payload_sha256,'softhsm-test') AS c
    WHERE c.canonical_payload::jsonb #> '{subject,ballot_publication_id}' = to_jsonb(t.publication_id)
      AND NOT ((c.canonical_payload::jsonb->'subject') ? 'publication_id')
);
EXECUTE check_prepare('wrong signed publication alias', :'issuer', 'softhsm-test', :'actor', false);
ROLLBACK TO wrong_alias;
SAVEPOINT mixed_alias;
EXECUTE change_signed_text('"publication_id":',
    '"ballot_publication_id":' || to_jsonb(:'publication'::uuid)::text || ',"publication_id":');
SET LOCAL SESSION AUTHORIZATION :"local_login";
EXECUTE check_prepare('both signed publication keys', :'issuer', 'softhsm-test', :'actor', false);
ROLLBACK TO mixed_alias;
SAVEPOINT other_post;
EXECUTE change_signed_text(:'post'::uuid::text, gen_random_uuid()::text);
SET LOCAL SESSION AUTHORIZATION :"local_login";
EXECUTE check_prepare('different signed Post', :'issuer', 'softhsm-test', :'actor', false);
ROLLBACK TO other_post;
SAVEPOINT other_publication;
EXECUTE change_signed_text(:'publication'::uuid::text, gen_random_uuid()::text);
SET LOCAL SESSION AUTHORIZATION :"local_login";
INSERT INTO pg_temp.local_checks SELECT 'other publication passes original context', EXISTS (
    SELECT FROM pg_temp.local_case AS t CROSS JOIN LATERAL sequent_backend.operational_initialize_context(
        t.tenant_id,t.event_id,t.request_id,t.actor,t.payload_sha256,'softhsm-test')
);
EXECUTE check_prepare('different signed publication', :'issuer', 'softhsm-test', :'actor', false);
ROLLBACK TO other_publication;

SAVEPOINT superseded;
-- Restore the newer Step publication while its predecessor and styles stay live.
WITH restored AS (
    UPDATE sequent_backend.ballot_publication AS newer SET deleted_at = NULL
    FROM pg_temp.local_case AS t JOIN sequent_backend.ballot_publication AS old
      ON (old.tenant_id, old.election_event_id, old.id) = (t.tenant_id, t.event_id, t.publication_id)
    WHERE (newer.tenant_id, newer.election_event_id, newer.id) = (t.tenant_id, t.event_id, :'newer_publication'::uuid)
      AND old.deleted_at IS NULL AND newer.deleted_at IS NOT NULL AND newer.published_at > old.published_at
      AND (newer.election_id = t.post_id OR t.post_id = ANY(newer.election_ids))
    RETURNING newer.id
)
INSERT INTO pg_temp.local_checks SELECT 'a newer matching publication is live', EXISTS (SELECT FROM restored);
SET LOCAL SESSION AUTHORIZATION :"local_login";
INSERT INTO pg_temp.local_checks SELECT 'superseded installation lookup',
    sequent_backend.operational_signing_local_initialize_installation_id(
        tenant_id, event_id, post_id, publication_id, 'softhsm-test') IS NULL FROM pg_temp.local_case;
EXECUTE check_prepare('superseded live publication', :'issuer', 'softhsm-test', :'actor', false);
ROLLBACK TO superseded;

SAVEPOINT tombstoned;
UPDATE sequent_backend.ballot_publication AS p SET deleted_at = clock_timestamp()
FROM pg_temp.local_case AS t WHERE (p.tenant_id,p.election_event_id,p.id) = (t.tenant_id,t.event_id,t.publication_id);
SET LOCAL SESSION AUTHORIZATION :"local_login";
EXECUTE check_prepare('tombstoned publication', :'issuer', 'softhsm-test', :'actor', false);
ROLLBACK TO tombstoned;

SAVEPOINT prepared;
SET LOCAL SESSION AUTHORIZATION :"local_login";
EXECUTE check_prepare('authorized signed subject.publication_id', :'issuer', 'softhsm-test', :'actor', true);
EXECUTE check_prepare('challenge replay', :'issuer', 'softhsm-test', :'actor', false);
RESET SESSION AUTHORIZATION;
INSERT INTO pg_temp.local_checks SELECT 'exact stored evidence', EXISTS (
    SELECT FROM pg_temp.local_case AS t
    JOIN sequent_backend.operational_initialize_attempt AS a ON a.challenge_id = t.attempt
    JOIN sequent_backend.operational_signing_intent AS i ON i.challenge_id = a.challenge_id
    JOIN sequent_backend.operational_signing_ceremony AS c ON (c.issuer, c.nonce_hash) = (i.issuer, i.nonce_hash)
    WHERE a.step = 'issued' AND a.device_profile = 'softhsm-test' AND i.phase = 'prepared'
      AND (i.tenant_id, i.event_id, i.request_id, i.actor, i.issuer, i.nonce_hash)
        = (t.tenant_id, t.event_id, t.request_id, t.actor, t.issuer, t.nonce)
      AND (i.binding).binding_sha256 = t.binding AND (i.binding).payload_hash = t.payload_sha256
);
SET LOCAL ROLE sequent_os_ledger_data_owner;
DO $$ BEGIN
    DELETE FROM sequent_backend.operational_initialize_local_profile;
    RAISE EXCEPTION 'retired a scope with pending evidence';
EXCEPTION WHEN object_in_use THEN NULL;
END $$;
ROLLBACK TO prepared;
INSERT INTO pg_temp.local_checks SELECT 'rolled back the ceremony, intent and attempt',
    NOT EXISTS (SELECT FROM sequent_backend.operational_signing_ceremony WHERE (issuer, nonce_hash) = (t.issuer, t.nonce))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_signing_intent WHERE challenge_id = t.attempt)
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_attempt WHERE challenge_id = t.attempt)
FROM pg_temp.local_case AS t;
SET LOCAL ROLE sequent_os_ledger_data_owner;
DELETE FROM sequent_backend.operational_initialize_local_profile;
RESET ROLE;
CREATE TEMP TABLE local_prepare_acl ON COMMIT DROP AS
    SELECT oid, proowner, proacl, prosecdef, proconfig FROM pg_catalog.pg_proc
    WHERE oid = 'sequent_backend.operational_initialize_prepare(uuid,uuid,uuid,text,text,text,uuid,text,text,text)'::regprocedure;
\ir down.sql
INSERT INTO pg_temp.local_checks SELECT 'prepare identity, owner, ACL and search path survive down', EXISTS (
    SELECT FROM pg_temp.local_prepare_acl AS old JOIN pg_catalog.pg_proc AS p USING (oid)
    WHERE (p.proowner,p.proacl,p.prosecdef,p.proconfig) IS NOT DISTINCT FROM
          (old.proowner,old.proacl,old.prosecdef,old.proconfig)
);
SET LOCAL SESSION AUTHORIZATION :"local_login";
EXECUTE check_prepare('restored HTTP refusal', :'issuer', 'softhsm-test', :'actor', false);
RESET SESSION AUTHORIZATION;
SET LOCAL SESSION AUTHORIZATION :"https_login";
EXECUTE check_prepare('restored HTTPS prepare', 'https://issuer.invalid', 'softhsm-test', :'actor', true);
ROLLBACK;

-- Two-session follow-up on a disposable copy with the profile committed.
-- A holds an uncommitted deployment promotion. B's exact local prepare must
-- wait on the shared policy lock, then refuse after A commits. In the reverse
-- order, hold B's successful prepare open and verify A cannot promote until
-- B ends. With a publisher holding the ballot row first, B may return 55P03.

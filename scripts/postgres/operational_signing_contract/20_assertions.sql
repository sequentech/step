-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Contract assertions. Each block runs as the real login of one principal
-- (SET SESSION AUTHORIZATION changes session_user and current_user, exactly
-- like a direct connection), so the callers' own start-up checks, which need
-- current_user = session_user, can be checked here.
\set ON_ERROR_STOP 1
\set QUIET 1
\pset tuples_only on

\set tenant_a 'a0000000-0000-4000-8000-000000000001'
\set tenant_b 'b0000000-0000-4000-8000-000000000001'
\set tenant_l 'c0000000-0000-4000-8000-000000000001'
\set event_a 'a0000000-0000-4000-8000-000000000002'
\set event_b 'b0000000-0000-4000-8000-000000000002'
\set event_l 'c0000000-0000-4000-8000-000000000002'
\set election_a 'a0000000-0000-4000-8000-000000000003'
\set area_a 'a0000000-0000-4000-8000-000000000004'
\set tally_a 'a0000000-0000-4000-8000-000000000005'
\set official 'a0000000-0000-4000-8000-0000000000a1'
\set officer 'a0000000-0000-4000-8000-0000000000a2'
\set officer2 'a0000000-0000-4000-8000-0000000000a3'
\set official3 'a0000000-0000-4000-8000-0000000000a4'
\set req1 'a0000000-0000-4000-8000-0000000000f1'
\set req2 'a0000000-0000-4000-8000-0000000000f2'
\set req3 'a0000000-0000-4000-8000-0000000000f3'
\set reqb 'b0000000-0000-4000-8000-0000000000f1'
\set reql 'c0000000-0000-4000-8000-0000000000f1'
\set gen1 'a0000000-0000-4000-8000-0000000000e1'
\set gen2 'a0000000-0000-4000-8000-0000000000e2'
\set gen3 'a0000000-0000-4000-8000-0000000000e3'
\set cert1 'a0000000-0000-4000-8000-0000000000d1'
\set t1 'a0000000-0000-4000-8000-000000000101'
\set t2 'a0000000-0000-4000-8000-000000000102'
\set t3 'a0000000-0000-4000-8000-000000000103'
\set t4 'a0000000-0000-4000-8000-000000000104'
\set issuer 'https://idp.invalid/realms/tenant-a0000000-0000-4000-8000-000000000001'
\set csr '-----BEGIN CERTIFICATE REQUEST-----\nTUlJQ0FUQ0NBZWtDQVFBd1h6RUxNQWtHQTFVRUJoTUNWVk14\n-----END CERTIFICATE REQUEST-----\n'
SELECT repeat('5a', 32) AS spki, repeat('e1', 32) AS evidence, repeat('f1', 32) AS fingerprint,
       repeat('01', 32) AS n1, repeat('02', 32) AS n2, repeat('03', 32) AS n3,
       repeat('04', 32) AS n4, repeat('b1', 32) AS h1, repeat('b2', 32) AS h2,
       repeat('b3', 32) AS h3, repeat('b4', 32) AS h4 \gset
SELECT payload_sha256 AS pay1 FROM sequent_backend.signing_request WHERE id = :'req1' \gset
SELECT payload_sha256 AS pay2 FROM sequent_backend.signing_request WHERE id = :'req2' \gset
SELECT payload_sha256 AS payb FROM sequent_backend.signing_request WHERE id = :'reqb' \gset
SELECT payload_sha256 AS payl FROM sequent_backend.signing_request WHERE id = :'reql' \gset

-- ---------------------------------------------------------------------------
-- 0. Catalogue: roles, owners, definer settings, nothing for PUBLIC.
-- ---------------------------------------------------------------------------
SELECT os_test.ok(
    (SELECT count(*) = 9 AND bool_and(NOT rolcanlogin AND NOT rolsuper AND NOT rolcreaterole
        AND NOT rolcreatedb AND NOT rolreplication AND NOT rolbypassrls)
     FROM pg_roles WHERE rolname LIKE 'sequent\_os\_%'),
    'nine NOLOGIN group roles without administrative attributes');
SELECT os_test.ok(
    (SELECT bool_and(NOT has_function_privilege('public', p.oid, 'EXECUTE'))
     FROM pg_proc p WHERE p.pronamespace = 'sequent_backend'::regnamespace
       AND p.proname LIKE 'operational\_%'),
    'no operational function is executable by PUBLIC');
SELECT os_test.ok(
    (SELECT bool_and(NOT has_table_privilege('public', c.oid,
        'SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER'))
     FROM pg_class c WHERE c.relnamespace = 'sequent_backend'::regnamespace
       AND c.relkind = 'r' AND c.relname LIKE 'operational\_%'),
    'no operational table privilege for PUBLIC');
SELECT os_test.ok(
    (SELECT count(*) = 7 FROM pg_class c
     WHERE c.relnamespace = 'sequent_backend'::regnamespace AND c.relkind = 'r'
       AND c.relowner = 'sequent_os_ledger_data_owner'::regrole
       AND c.relname IN ('operational_initialize_attempt', 'operational_signing_device_grant',
           'operational_setup_installation', 'operational_setup_generation',
           'operational_miru_inventory', 'operational_miru_artifact',
           'operational_miru_revision_artifact'))
    AND (SELECT count(*) = 7 FROM pg_class c
         WHERE c.relowner = 'sequent_os_ledger_data_owner'::regrole AND c.relkind = 'r'),
    'the seven new tables, and nothing else, belong to the ledger data owner');
-- Every function a caller role may execute is SECURITY DEFINER with the
-- pinned search_path.
SELECT os_test.ok(
    (SELECT count(*) = 27
        AND bool_and(p.prosecdef)
        AND bool_and(p.proconfig = ARRAY['search_path=pg_catalog, sequent_backend, pg_temp'])
        AND bool_and(p.proowner = 'sequent_os_ledger_api_owner'::regrole)
     FROM pg_proc p
     WHERE p.pronamespace = 'sequent_backend'::regnamespace
       AND p.proowner = 'sequent_os_ledger_api_owner'::regrole
       AND EXISTS (SELECT FROM aclexplode(p.proacl) a
                   WHERE a.grantee <> p.proowner AND a.grantee <> 0)),
    'the 27 granted contract functions are SECURITY DEFINER with the pinned search_path');
SELECT os_test.ok(
    (SELECT relowner::regrole::text <> 'sequent_os_ledger_data_owner' FROM pg_class
     WHERE oid = 'sequent_backend.operational_signing_intent'::regclass)
    AND (SELECT proowner::regrole::text NOT LIKE 'sequent\_os\_%' FROM pg_proc
         WHERE oid = 'sequent_backend.operational_signing_reserve(sequent_backend.operational_signing_binding,bigint,timestamptz)'::regprocedure),
    'objects of 0700-0900 keep their owner');

-- ---------------------------------------------------------------------------
-- 1. Setup generation and certificate attachment.
-- ---------------------------------------------------------------------------
SET SESSION AUTHORIZATION os_t_setup;
-- Contract: the setup authority works only through the functions; it has no
-- privilege on the setup or enrollment ledgers and no other operational role.
SELECT os_test.ok((
    SELECT current_user=session_user
     AND pg_has_role(current_user,'sequent_os_setup_authority','USAGE')
     AND NOT pg_has_role(current_user,'sequent_os_setup_device','MEMBER')
     AND NOT pg_has_role(current_user,'sequent_os_ledger_api_owner','MEMBER')
     AND NOT pg_has_role(current_user,'sequent_os_ledger_data_owner','MEMBER')
     AND NOT pg_has_role(current_user,'sequent_os_initialize_ca','MEMBER')
     AND NOT pg_has_role(current_user,'sequent_os_initialize_device','MEMBER')
     AND NOT EXISTS(SELECT FROM pg_roles r WHERE r.rolname=current_user AND (r.rolsuper OR r.rolcreaterole OR r.rolbypassrls))
     AND NOT has_table_privilege(current_user,'sequent_backend.operational_setup_generation','SELECT,INSERT,UPDATE,DELETE')
     AND NOT has_any_column_privilege(current_user,'sequent_backend.operational_setup_generation','SELECT,INSERT,UPDATE,REFERENCES')
     AND NOT has_table_privilege(current_user,'sequent_backend.operational_signing_official_enrollment','SELECT,INSERT,UPDATE,DELETE')
     AND NOT has_any_column_privilege(current_user,'sequent_backend.operational_signing_official_enrollment','SELECT,INSERT,UPDATE,REFERENCES')),
    'setup authority holds only its own role and no ledger privilege');
SELECT os_test.ok(
    (SELECT (phase, attempt) IS NOT DISTINCT FROM ('unavailable', NULL::uuid)
     FROM sequent_backend.operational_setup_management_status(:'tenant_a', :'event_a', :'official', :'officer')),
    'setup is unavailable before the tenant has an installed device profile');
SELECT os_test.ok(
    NOT sequent_backend.operational_setup_prepare_generation(:'tenant_a', :'event_a', :'official', :'officer', :'gen1'),
    'no generation without an installation');
SELECT os_test.denied(format(
    'INSERT INTO sequent_backend.operational_setup_installation (tenant_id, device_profile) VALUES (%L, %L)',
    :'tenant_a', 'yubihsm'), 'setup authority cannot provision the installation');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_admin;
SELECT os_test.refused(format(
    'INSERT INTO sequent_backend.operational_setup_installation (tenant_id, device_profile) VALUES (%L, %L)',
    :'tenant_l', 'yubihsm'), 'no installation for a legacy tenant');
INSERT INTO sequent_backend.operational_setup_installation (tenant_id, device_profile)
VALUES (:'tenant_a', 'yubihsm');
SELECT os_test.refused(format(
    'UPDATE sequent_backend.operational_setup_installation SET device_profile = %L WHERE tenant_id = %L',
    'softhsm-test', :'tenant_a'), 'an installed device profile cannot be changed');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_setup_prepare_generation(%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'official', :'officer', :'gen1'),
    'ledger data owner cannot act as the setup authority');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_setup;
SELECT os_test.ok(
    (SELECT (phase, attempt) IS NOT DISTINCT FROM ('not-started', NULL::uuid)
     FROM sequent_backend.operational_setup_management_status(:'tenant_a', :'event_a', :'official', :'officer')),
    'setup not started');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_setup_management_status(:'tenant_a', :'event_a', :'official', :'official')),
    'an official is not their own setup authority (status)');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_setup_management_status(:'tenant_b', :'event_a', :'official', :'officer')),
    'cross-tenant event has no setup status');
SELECT os_test.ok(
    (SELECT phase = 'unavailable' FROM sequent_backend.operational_setup_management_status(:'tenant_l', :'event_l', :'official', :'officer')),
    'setup is unavailable for a legacy tenant');
SELECT os_test.ok(
    NOT sequent_backend.operational_setup_prepare_generation(:'tenant_a', :'event_a', :'official', :'official', :'gen1'),
    'an official cannot prepare their own key generation');
SELECT os_test.ok(
    NOT sequent_backend.operational_setup_prepare_generation(:'tenant_a', :'event_b', :'official', :'officer', :'gen1'),
    'no generation for another tenant''s event');
SELECT os_test.ok(
    NOT sequent_backend.operational_setup_prepare_generation(:'tenant_l', :'event_l', :'official', :'officer', :'gen1'),
    'no generation for a legacy tenant');
SELECT os_test.ok(
    sequent_backend.operational_setup_prepare_generation(:'tenant_a', :'event_a', :'official', :'officer', :'gen1'),
    'setup authority prepares the generation');
SELECT os_test.ok(
    NOT sequent_backend.operational_setup_prepare_generation(:'tenant_a', :'event_a', :'official', :'officer', :'gen1')
    AND NOT sequent_backend.operational_setup_prepare_generation(:'tenant_a', :'event_a', :'official', :'officer2', :'gen2'),
    'a second generation for the same official is refused (replay and new attempt)');
SELECT os_test.ok(
    (SELECT (phase, attempt) = ('pending', :'gen1'::uuid)
     FROM sequent_backend.operational_setup_management_status(:'tenant_a', :'event_a', :'official', :'officer')),
    'prepared generation is pending');
SELECT os_test.denied(format(
    'SELECT * FROM sequent_backend.operational_setup_claim_generation(%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'official', :'gen1'), 'setup authority cannot claim the device generation');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_setup_complete_generation(%L,%L,%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'official', :'gen1', :'gen1', :'spki', :'evidence', :'csr'),
    'setup authority cannot complete the device generation');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_signing_enroll_official(%L,%L,%L,1,%L,%L,true,%L)',
    :'tenant_a', :'official', 'k', :'spki', 'yubihsm', :'evidence'),
    'setup authority cannot enroll a key directly');
SELECT os_test.denied('SELECT * FROM sequent_backend.operational_setup_generation',
    'setup authority cannot read the generation table');
RESET SESSION AUTHORIZATION;

-- A generation whose claim window has passed (synthetic old row).
INSERT INTO sequent_backend.operational_setup_generation
    (attempt, tenant_id, event_id, official, prepared_by, device_profile, prepared_at, claim_deadline)
VALUES (:'gen3', :'tenant_a', :'event_a', :'official3', :'officer', 'yubihsm',
        clock_timestamp() - interval '10 minutes', clock_timestamp() - interval '8 minutes');

SET SESSION AUTHORIZATION os_t_setup_device;
-- Contract: the setup device holds only its own role, no ledger privilege and
-- no enrollment function.
SELECT os_test.ok((
    SELECT current_user=session_user AND
      pg_has_role(current_user,'sequent_os_setup_device','USAGE') AND
      NOT pg_has_role(current_user,'sequent_os_setup_authority','MEMBER') AND
      NOT pg_has_role(current_user,'sequent_os_ledger_api_owner','MEMBER') AND
      NOT pg_has_role(current_user,'sequent_os_ledger_data_owner','MEMBER') AND
      NOT pg_has_role(current_user,'sequent_os_approve','MEMBER') AND
      NOT pg_has_role(current_user,'sequent_os_initialize_device','MEMBER') AND
      NOT has_table_privilege(current_user,'sequent_backend.operational_setup_generation','SELECT,INSERT,UPDATE,DELETE') AND
      NOT has_any_column_privilege(current_user,'sequent_backend.operational_setup_generation','SELECT,INSERT,UPDATE,REFERENCES') AND
      NOT has_function_privilege(current_user,'sequent_backend.operational_signing_enroll_official(uuid,text,text,integer,text,text,boolean,text)','EXECUTE')),
    'setup device holds only its own role and no ledger privilege');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_setup_prepare_generation(%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'official3', :'officer', :'gen2'), 'setup device cannot prepare a generation');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_setup_public_csr(%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'official', :'officer', :'gen1'), 'setup device cannot read the CSR route');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_setup_claim_generation(:'tenant_b', :'event_a', :'official', :'gen1'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_setup_claim_generation(:'tenant_a', :'event_b', :'official', :'gen1'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_setup_claim_generation(:'tenant_a', :'event_a', :'official3', :'gen1')),
    'claim refuses another tenant, event or official');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_setup_claim_generation(:'tenant_a', :'event_a', :'official3', :'gen3')),
    'claim refuses an attempt whose claim window expired');
SELECT fence AS gen_fence, deadline > clock_timestamp() AS gen_live
FROM sequent_backend.operational_setup_claim_generation(:'tenant_a', :'event_a', :'official', :'gen1') \gset
SELECT os_test.ok(:'gen_fence' IS NOT NULL AND :'gen_live'::boolean, 'setup device claims the generation once');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_setup_claim_generation(:'tenant_a', :'event_a', :'official', :'gen1')),
    'a claimed generation cannot be claimed again');
SELECT os_test.ok(
    sequent_backend.operational_setup_complete_generation(:'tenant_a', :'event_a', :'official', :'gen1', :'gen2', :'spki', :'evidence', :'csr') IS NULL,
    'completion refuses a wrong fence');
SELECT os_test.ok(
    sequent_backend.operational_setup_complete_generation(:'tenant_a', :'event_a', :'official', :'gen1', :'gen_fence',
        :'spki', :'evidence', E'-----BEGIN PRIVATE KEY-----\nTUlJQ0FUQ0NBZWtDQVFBd1h6RUxNQWtHQTFVRUJoTUNWVk14\n-----END PRIVATE KEY-----\n') IS NULL
    AND sequent_backend.operational_setup_complete_generation(:'tenant_a', :'event_a', :'official', :'gen1', :'gen_fence',
        'not-a-digest', :'evidence', :'csr') IS NULL,
    'completion refuses anything but a public CSR and digests');
SELECT os_test.ok(
    sequent_backend.operational_setup_complete_generation(:'tenant_b', :'event_a', :'official', :'gen1', :'gen_fence', :'spki', :'evidence', :'csr') IS NULL,
    'completion refuses another tenant');
SELECT sequent_backend.operational_setup_complete_generation(:'tenant_a', :'event_a', :'official', :'gen1', :'gen_fence', :'spki', :'evidence', :'csr') AS enrollment \gset
SELECT os_test.ok(:'enrollment' IS NOT NULL, 'setup device completes the generation and the key is enrolled');
SELECT os_test.ok(
    sequent_backend.operational_setup_complete_generation(:'tenant_a', :'event_a', :'official', :'gen1', :'gen_fence', :'spki', :'evidence', :'csr') IS NULL,
    'a completed generation cannot be completed again');
RESET SESSION AUTHORIZATION;

SELECT os_test.ok(
    (SELECT (e.actor, e.key_id, e.key_version, e.spki, e.device_profile, e.nonexportable_confirmed, e.device_evidence_hash)
          = (:'official', 'official/' || :'tenant_a' || '/' || :'official', 1, :'spki', 'yubihsm', true, :'evidence')
     FROM sequent_backend.operational_signing_official_enrollment e WHERE e.id = :'enrollment'),
    'the enrollment carries the installed profile, version 1 and the official key id');

-- What Harvest's ordinary staff-certificate registration writes in the same
-- transaction as the attachment (synthetic, not a parseable certificate).
INSERT INTO sequent_backend.staff_certificate
    (id, tenant_id, election_event_id, user_id, username, fingerprint_sha256, spki_sha256,
     holder_sha256, serial, subject, issuer, not_before, not_after, pem, registration, registered_by)
VALUES (:'cert1', :'tenant_a', :'event_a', :'official', 'official', :'fingerprint', :'spki',
        repeat('ab', 32), '02', 'CN=Official', 'CN=Test staff CA', now() - interval '1 hour',
        now() + interval '30 days', 'TEST-LEAF-PEM', 'security-officer', :'officer');

SET SESSION AUTHORIZATION os_t_setup;
SELECT os_test.ok(
    (SELECT (phase, attempt) = ('certificate-request-issued', :'gen1'::uuid)
     FROM sequent_backend.operational_setup_management_status(:'tenant_a', :'event_a', :'official', :'officer')),
    'certificate request issued');
SELECT os_test.ok(
    (SELECT phase = 'reconcile' FROM sequent_backend.operational_setup_management_status(:'tenant_a', :'event_a', :'official3', :'officer')),
    'an unclaimed expired generation needs reconciliation');
SELECT os_test.ok(
    sequent_backend.operational_setup_public_csr(:'tenant_a', :'event_a', :'official', :'officer', :'gen1') = :'csr',
    'setup authority reads the public CSR');
SELECT os_test.ok(
    sequent_backend.operational_setup_public_csr(:'tenant_a', :'event_a', :'official', :'official', :'gen1') IS NULL
    AND sequent_backend.operational_setup_public_csr(:'tenant_b', :'event_a', :'official', :'officer', :'gen1') IS NULL
    AND sequent_backend.operational_setup_public_csr(:'tenant_a', :'event_a', :'official', :'officer', :'gen2') IS NULL,
    'CSR refused for the official, another tenant or another attempt');
SELECT os_test.ok(
    (SELECT (enrollment_id, expected_spki) = (:'enrollment'::uuid, :'spki')
     FROM sequent_backend.operational_setup_pin_for_attachment(:'tenant_a', :'event_a', :'official', :'officer', :'gen1')),
    'setup authority pins the enrollment for attachment');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_setup_pin_for_attachment(:'tenant_a', :'event_a', :'official', :'official', :'gen1'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_setup_pin_for_attachment(:'tenant_b', :'event_a', :'official', :'officer', :'gen1')),
    'no pin for the official or another tenant');
SELECT os_test.ok(
    NOT sequent_backend.operational_setup_attach_certificate(:'tenant_a', :'event_a', :'official', :'officer2', :'gen1',
        :'enrollment', :'cert1', :'fingerprint', ARRAY['TEST-LEAF-PEM']),
    'attachment refused for an officer who did not register the certificate');
SELECT os_test.ok(
    NOT sequent_backend.operational_setup_attach_certificate(:'tenant_a', :'event_a', :'official', :'official', :'gen1',
        :'enrollment', :'cert1', :'fingerprint', ARRAY['TEST-LEAF-PEM']),
    'attachment refused when the official acts as authority');
SELECT os_test.ok(
    NOT sequent_backend.operational_setup_attach_certificate(:'tenant_a', :'event_a', :'official', :'officer', :'gen1',
        :'enrollment', :'cert1', :'fingerprint', ARRAY['OTHER-LEAF-PEM']),
    'attachment refused when the chain leaf is not the registered certificate');
SELECT os_test.ok(
    sequent_backend.operational_setup_attach_certificate(:'tenant_a', :'event_a', :'official', :'officer', :'gen1',
        :'enrollment', :'cert1', :'fingerprint', ARRAY['TEST-LEAF-PEM', 'TEST-ANCHOR-PEM']),
    'setup authority attaches the certificate');
SELECT os_test.ok(
    NOT sequent_backend.operational_setup_attach_certificate(:'tenant_a', :'event_a', :'official', :'officer', :'gen1',
        :'enrollment', :'cert1', :'fingerprint', ARRAY['TEST-LEAF-PEM', 'TEST-ANCHOR-PEM']),
    'attachment cannot be replayed');
SELECT os_test.ok(
    (SELECT (phase, attempt) = ('certificate-attached', :'gen1'::uuid)
     FROM sequent_backend.operational_setup_management_status(:'tenant_a', :'event_a', :'official', :'officer')),
    'certificate attached');
SELECT os_test.ok(
    sequent_backend.operational_setup_public_csr(:'tenant_a', :'event_a', :'official', :'officer', :'gen1') IS NULL,
    'no current CSR after attachment');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_official_setup_status(%L,%L,%L)', :'tenant_a', :'event_a', :'official'),
    'setup authority cannot use the approval caller''s status');
RESET SESSION AUTHORIZATION;

-- ---------------------------------------------------------------------------
-- 2. Approval caller (Harvest/Windmill): own status and start-up shape.
-- ---------------------------------------------------------------------------
SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok((
    SELECT current_user = session_user
       AND pg_has_role(current_user, 'sequent_os_approve', 'USAGE')
       AND NOT EXISTS (SELECT FROM pg_roles r WHERE r.rolname LIKE 'sequent\_os\_%'
                       AND r.rolname <> 'sequent_os_approve' AND pg_has_role(current_user, r.oid, 'MEMBER'))
       AND NOT has_table_privilege(current_user, 'sequent_backend.operational_signing_intent',
                                   'SELECT,INSERT,UPDATE,DELETE,TRUNCATE,TRIGGER')
       AND NOT has_any_column_privilege(current_user, 'sequent_backend.operational_signing_intent',
                                        'INSERT,UPDATE,REFERENCES')
       AND NOT EXISTS (SELECT FROM pg_attribute a
           WHERE a.attrelid = 'sequent_backend.operational_signing_intent'::regclass
             AND a.attnum > 0 AND NOT a.attisdropped
             AND a.attname NOT IN ('tenant_id','event_id','request_id','actor','phase','purpose',
                 'binding','receipt','reservation_id','applied_signature_sha256',
                 'applied_pdf_revision_id','applied_pdf_document_id','applied_pdf_sha256')
             AND has_column_privilege(current_user, a.attrelid, a.attnum, 'SELECT'))),
    'approval caller has only the applied-evidence column reads on the intent ledger');
SELECT os_test.ok(
    sequent_backend.operational_official_setup_status(:'tenant_a', :'event_a', :'official') = 'certificate-on-file'
    AND sequent_backend.operational_official_setup_status(:'tenant_a', :'event_a', :'officer') = 'key-not-enrolled',
    'official setup status: certificate on file / key not enrolled');
SELECT os_test.ok(
    sequent_backend.operational_official_setup_status(:'tenant_l', :'event_l', :'official') IS NULL
    AND sequent_backend.operational_official_setup_status(:'tenant_b', :'event_a', :'official') IS NULL,
    'official setup status is out of scope for a legacy tenant and a cross-tenant event');
SELECT os_test.ok(
    (SELECT mode = 'legacy-p12' FROM sequent_backend.operational_signing_policy WHERE singleton)
    AND EXISTS (SELECT FROM sequent_backend.operational_signing_required_tenant WHERE tenant_id = :'tenant_a'),
    'approval caller reads the signing policy');
SELECT os_test.denied(format(
    'SELECT * FROM sequent_backend.operational_setup_management_status(%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'official', :'officer'), 'approval caller is not the setup authority');
SELECT os_test.denied(format(
    'SELECT * FROM sequent_backend.operational_initialize_context(%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'pay1', 'yubihsm'), 'approval caller cannot read the broker context');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_initialize_prepare(%L,%L,%L,%L,%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'pay1', 'yubihsm', :'t1', :'issuer', :'n1', :'h1'),
    'approval caller cannot create a challenge');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_nobody;
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_official_setup_status(%L,%L,%L)', :'tenant_a', :'event_a', :'official'),
    'a login without a group role cannot call the contract');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_miru_generic_withhold(%L,%L,%L)', :'tenant_a', :'event_a', :'official'),
    'a login without a group role cannot call the MIRU gate');
SELECT os_test.denied('SELECT * FROM sequent_backend.operational_initialize_attempt',
    'a login without a group role cannot read the attempt table');
RESET SESSION AUTHORIZATION;

-- ---------------------------------------------------------------------------
-- 3. Initialize broker: context, challenge, callback, reservation.
-- ---------------------------------------------------------------------------
SET SESSION AUTHORIZATION os_t_broker;
-- Contract: the physical-side principals have no relation privilege at all.
SELECT os_test.ok((
    SELECT current_user = session_user
       AND pg_has_role(current_user, 'sequent_os_initialize_orchestrator', 'USAGE')
       AND NOT EXISTS (SELECT FROM pg_roles r
                       WHERE r.rolname::text NOT IN ('os_t_broker', 'sequent_os_initialize_orchestrator')
                         AND pg_has_role(current_user, r.rolname, 'MEMBER'))
       AND NOT has_schema_privilege(current_user, 'sequent_backend', 'CREATE')
       AND NOT EXISTS (SELECT FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
           WHERE n.nspname = 'sequent_backend' AND c.relkind IN ('r','p','v','m','f')
             AND (has_table_privilege(current_user, c.oid, 'SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER,MAINTAIN')
                  OR has_any_column_privilege(current_user, c.oid, 'SELECT,INSERT,UPDATE,REFERENCES')))),
    'broker login has the orchestrator role and no relation privileges');
SELECT os_test.ok(
    (SELECT count(*) = 1 AND bool_and(payload_sha256 = :'pay1' AND key_version = 1
        AND key_id = 'official/' || :'tenant_a' || '/' || :'official'
        AND cert_fingerprint = :'fingerprint' AND cert_spki = :'spki'
        AND issued_at < intent_expiry AND intent_expiry <= request_expiry
        AND intent_expiry <= issued_at + interval '240 seconds'
        AND encode(sha256(convert_to(canonical_payload, 'UTF8')), 'hex') = :'pay1')
     FROM sequent_backend.operational_initialize_context(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', 'yubihsm')),
    'broker reads the exact Initialize context');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_context(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', 'softhsm-test')),
    'no context for a device profile the key was not enrolled under');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_context(:'tenant_b', :'event_a', :'req1', :'official', :'pay1', 'yubihsm'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_context(:'tenant_a', :'event_b', :'reqb', :'official', :'payb', 'yubihsm'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_context(:'tenant_b', :'event_b', :'reqb', :'official', :'payb', 'yubihsm')),
    'no context across tenants or for a tenant where the official has no key');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_context(:'tenant_l', :'event_l', :'reql', :'official', :'payl', 'yubihsm')),
    'no context for a legacy tenant');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_context(:'tenant_a', :'event_a', :'req1', :'officer', :'pay1', 'yubihsm'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_context(:'tenant_a', :'event_a', :'req1', :'official', :'pay2', 'yubihsm')),
    'no context for an actor without a key or for another payload');
BEGIN;
SELECT count(*) AS seen FROM sequent_backend.operational_initialize_context(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', 'yubihsm') \gset
SELECT sequent_backend.operational_initialize_prepare(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', 'yubihsm', :'t1', :'issuer', :'n1', :'h1') AS prepared \gset
COMMIT;
SELECT os_test.ok(:seen = 1 AND :'prepared'::boolean, 'broker creates the challenge in the context transaction');
SELECT os_test.ok(
    NOT sequent_backend.operational_initialize_prepare(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', 'yubihsm', :'t1', :'issuer', :'n1', :'h1')
    AND NOT sequent_backend.operational_initialize_prepare(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', 'yubihsm', :'t2', :'issuer', :'n2', :'h2'),
    'a second challenge for the same actor and request is refused (replay and new attempt)');
SELECT os_test.ok(
    NOT sequent_backend.operational_initialize_prepare(:'tenant_a', :'event_a', :'req2', :'official', :'pay2', 'yubihsm', :'t2', 'http://idp.invalid/x', :'n2', :'h2')
    AND NOT sequent_backend.operational_initialize_prepare(:'tenant_a', :'event_a', :'req2', :'official', :'pay2', 'softhsm-test', :'t2', :'issuer', :'n2', :'h2')
    AND NOT sequent_backend.operational_initialize_prepare(:'tenant_a', :'event_a', :'req2', :'official', :'pay2', 'yubihsm', :'t2', :'issuer', :'n1', :'h2')
    AND NOT sequent_backend.operational_initialize_prepare(:'tenant_b', :'event_a', :'req2', :'official', :'pay2', 'yubihsm', :'t2', :'issuer', :'n2', :'h2'),
    'challenge refused for a non-HTTPS issuer, wrong profile, reused nonce or another tenant');
SELECT os_test.ok(
    (SELECT count(*) = 1 AND bool_and(phase = 'prepared'
        AND binding->>'challenge_id' = :'t1' AND binding->>'actor' = :'official'
        AND binding->>'action' = 'initialize-voting' AND binding->>'purpose' = 'approval-payload'
        AND binding->>'payload_hash' = :'pay1' AND binding->>'message_hash' = :'pay1'
        AND binding->>'artifact_hash' = :'pay1' AND binding->>'binding_sha256' = :'h1'
        AND binding->>'nonce_hash' = :'n1' AND binding->>'issuer' = :'issuer'
        AND (binding->>'key_version')::int = 1
        AND (binding->>'request_expires_at')::timestamptz >= (binding->>'intent_expires_at')::timestamptz
        AND encode(sha256(convert_to(canonical_payload, 'UTF8')), 'hex') = :'pay1')
     FROM sequent_backend.operational_initialize_load(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', :'t1')),
    'broker loads its exact stored challenge');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_load(:'tenant_a', :'event_a', :'req1', :'officer', :'pay1', :'t1'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_load(:'tenant_b', :'event_a', :'req1', :'official', :'pay1', :'t1'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_load(:'tenant_a', :'event_a', :'req2', :'official', :'pay2', :'t1'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_load(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', :'t2')),
    'load never returns another actor, tenant, request or attempt');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_initialize_record_browser_return(%L,%L,%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'t1', 'election-show', 'publish', :'election_a'),
    'broker cannot record the browser return');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_initialize_discard_unredeemed(%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'pay1', :'t1'), 'broker cannot cancel for the approval caller');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_initialize_claim_current_device(%L,%L,%L,%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'t1', :'t1', :'h1', 'yubihsm', :'h1'),
    'broker cannot claim the device');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_signing_settle_device_from_reference(%L,%L,%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'t1', :'t1', :'h1', '\x3006020101020101'),
    'broker cannot settle a device receipt');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_signing_reserve(NULL::sequent_backend.operational_signing_binding, %L, now())', 1),
    'broker cannot call the raw ledger reservation');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(
    (SELECT (phase, challenge, callback_claimed) = ('prepared', :'t1'::uuid, false)
     FROM sequent_backend.operational_initialize_current_state(:'tenant_a', :'event_a', :'req1', :'official', :'pay1')),
    'approval caller sees its prepared, unredeemed attempt');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_current_state(:'tenant_a', :'event_a', :'req1', :'officer', :'pay1'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_current_state(:'tenant_b', :'event_a', :'req1', :'official', :'pay1')),
    'no attempt state for another actor or tenant');
SELECT os_test.ok(
    NOT sequent_backend.operational_initialize_record_browser_return(:'tenant_a', :'event_a', :'req1', :'official', :'t1', 'https://elsewhere.invalid/', 'publish', NULL)
    AND NOT sequent_backend.operational_initialize_record_browser_return(:'tenant_a', :'event_a', :'req1', :'official', :'t1', 'election-show', 'signatures-requests', :'election_a')
    AND NOT sequent_backend.operational_initialize_record_browser_return(:'tenant_a', :'event_a', :'req1', :'official', :'t1', 'election-show', 'publish', :'area_a')
    AND NOT sequent_backend.operational_initialize_record_browser_return(:'tenant_a', :'event_a', :'req1', :'official', :'t1', 'event-show', 'publish', :'election_a')
    AND NOT sequent_backend.operational_initialize_record_browser_return(:'tenant_a', :'event_a', :'req1', :'officer', :'t1', 'event-show', 'publish', NULL)
    AND NOT sequent_backend.operational_initialize_record_browser_return(:'tenant_b', :'event_a', :'req1', :'official', :'t1', 'event-show', 'publish', NULL),
    'return page refused for a free path, a wrong Post, another actor or tenant');
SELECT os_test.ok(
    sequent_backend.operational_initialize_record_browser_return(:'tenant_a', :'event_a', :'req1', :'official', :'t1', 'election-show', 'publish', :'election_a'),
    'approval caller records the return page');
SELECT os_test.ok(
    NOT sequent_backend.operational_initialize_record_browser_return(:'tenant_a', :'event_a', :'req1', :'official', :'t1', 'event-show', 'publish', NULL),
    'the return page is recorded once');
SELECT os_test.ok(
    (SELECT (path_kind, panel, election_id) = ('election-show', 'publish', :'election_a'::uuid)
     FROM sequent_backend.operational_initialize_get_browser_return(:'tenant_a', :'event_a', :'req1', :'official', :'t1'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_get_browser_return(:'tenant_a', :'event_a', :'req1', :'officer', :'t1')),
    'approval caller reads back only its own return page');
SELECT os_test.ok(
    NOT sequent_backend.operational_initialize_discard_unredeemed(:'tenant_a', :'event_a', :'req1', :'officer', :'pay1', :'t1')
    AND NOT sequent_backend.operational_initialize_discard_unredeemed(:'tenant_b', :'event_a', :'req1', :'official', :'pay1', :'t1'),
    'another actor or tenant cannot cancel the attempt');
SELECT os_test.ok(
    sequent_backend.operational_initialize_discard_unredeemed(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', :'t1'),
    'approval caller cancels its unredeemed attempt');
SELECT os_test.ok(
    NOT sequent_backend.operational_initialize_discard_unredeemed(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', :'t1')
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_current_state(:'tenant_a', :'event_a', :'req1', :'official', :'pay1'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_get_browser_return(:'tenant_a', :'event_a', :'req1', :'official', :'t1')),
    'a cancelled attempt is gone for the approval caller');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_broker;
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_load(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', :'t1'))
    AND NOT sequent_backend.operational_initialize_take_callback(:'tenant_a', :'event_a', :'req1', :'official', :'t1', :'h1'),
    'a cancelled attempt cannot be loaded or redeemed');
SELECT os_test.ok(
    NOT sequent_backend.operational_initialize_prepare(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', 'yubihsm', :'t1', :'issuer', :'n2', :'h2')
    AND NOT sequent_backend.operational_initialize_prepare(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', 'yubihsm', :'t2', :'issuer', :'n1', :'h2'),
    'a cancelled attempt identifier and its issuer nonce stay burned');
SELECT os_test.ok(
    sequent_backend.operational_initialize_prepare(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', 'yubihsm', :'t2', :'issuer', :'n2', :'h2'),
    'broker creates a fresh challenge after the cancel');
SELECT os_test.ok(
    sequent_backend.operational_initialize_reserve_own_callback(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h2', :'n2',
        :'issuer', floor(extract(epoch FROM clock_timestamp()))::bigint, clock_timestamp() + interval '60 seconds', 'yubihsm') IS NULL,
    'no reservation before the callback was redeemed');
SELECT os_test.ok(
    NOT sequent_backend.operational_initialize_take_callback(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h1')
    AND NOT sequent_backend.operational_initialize_take_callback(:'tenant_b', :'event_a', :'req1', :'official', :'t2', :'h2')
    AND NOT sequent_backend.operational_initialize_take_callback(:'tenant_a', :'event_a', :'req1', :'officer', :'t2', :'h2'),
    'callback refused for a wrong binding, tenant or actor');
SELECT os_test.ok(
    sequent_backend.operational_initialize_take_callback(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h2'),
    'broker redeems the callback');
SELECT os_test.ok(
    NOT sequent_backend.operational_initialize_take_callback(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h2'),
    'the callback is redeemed once');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(
    (SELECT (phase, challenge, callback_claimed) = ('prepared', :'t2'::uuid, true)
     FROM sequent_backend.operational_initialize_current_state(:'tenant_a', :'event_a', :'req1', :'official', :'pay1'))
    AND NOT sequent_backend.operational_initialize_discard_unredeemed(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', :'t2')
    AND NOT sequent_backend.operational_initialize_record_browser_return(:'tenant_a', :'event_a', :'req1', :'official', :'t2', 'event-show', 'publish', NULL),
    'a redeemed attempt can no longer be cancelled or given a return page');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_initialize_reserve_own_callback(%L,%L,%L,%L,%L,%L,%L,%L,%L,now(),%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h2', :'n2', :'issuer', 1, 'yubihsm'),
    'approval caller cannot reserve the device');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_initialize_take_callback(%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h2'), 'approval caller cannot redeem a callback');
RESET SESSION AUTHORIZATION;

-- auth_time has second resolution and must be after the challenge was issued.
SELECT pg_sleep(1.1);
SET SESSION AUTHORIZATION os_t_broker;
SELECT os_test.ok(
    sequent_backend.operational_initialize_reserve_own_callback(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h2', :'n1',
        :'issuer', floor(extract(epoch FROM clock_timestamp()))::bigint, clock_timestamp() + interval '60 seconds', 'yubihsm') IS NULL
    AND sequent_backend.operational_initialize_reserve_own_callback(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h2', :'n2',
        'https://other.invalid/realms/x', floor(extract(epoch FROM clock_timestamp()))::bigint, clock_timestamp() + interval '60 seconds', 'yubihsm') IS NULL
    AND sequent_backend.operational_initialize_reserve_own_callback(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h2', :'n2',
        :'issuer', floor(extract(epoch FROM clock_timestamp()))::bigint, clock_timestamp() + interval '60 seconds', 'softhsm-test') IS NULL
    AND sequent_backend.operational_initialize_reserve_own_callback(:'tenant_b', :'event_a', :'req1', :'official', :'t2', :'h2', :'n2',
        :'issuer', floor(extract(epoch FROM clock_timestamp()))::bigint, clock_timestamp() + interval '60 seconds', 'yubihsm') IS NULL,
    'reservation refused for a wrong nonce, issuer, device profile or tenant');
SELECT os_test.ok(
    sequent_backend.operational_initialize_reserve_own_callback(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h2', :'n2',
        :'issuer', floor(extract(epoch FROM clock_timestamp() - interval '10 minutes'))::bigint, clock_timestamp() + interval '60 seconds', 'yubihsm') IS NULL
    AND sequent_backend.operational_initialize_reserve_own_callback(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h2', :'n2',
        :'issuer', floor(extract(epoch FROM clock_timestamp()))::bigint, clock_timestamp() - interval '1 second', 'yubihsm') IS NULL,
    'reservation refused for a stale authentication or an expired proof');
SELECT floor(extract(epoch FROM clock_timestamp()))::bigint AS auth_time \gset
SELECT sequent_backend.operational_initialize_reserve_own_callback(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h2', :'n2',
    :'issuer', :auth_time, clock_timestamp() + interval '60 seconds', 'yubihsm') AS fence \gset
SELECT os_test.ok(:'fence' IS NOT NULL, 'broker reserves the device for its own redeemed callback');
SELECT os_test.ok(
    sequent_backend.operational_initialize_reserve_own_callback(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h2', :'n2',
        :'issuer', :auth_time, clock_timestamp() + interval '60 seconds', 'yubihsm') IS NULL,
    'the reservation cannot be replayed');
SELECT os_test.ok(
    (SELECT phase = 'reserved' FROM sequent_backend.operational_initialize_load(:'tenant_a', :'event_a', :'req1', :'official', :'pay1', :'t2')),
    'broker sees the reserved phase');
SELECT os_test.denied(format(
    'SELECT * FROM sequent_backend.operational_signing_get_device_grant(%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence'), 'broker cannot read the device grant');
SELECT os_test.denied(format(
    'SELECT * FROM sequent_backend.operational_initialize_get_ca_enrollment(%L,%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', 'yubihsm'), 'broker cannot read the CA enrollment');
RESET SESSION AUTHORIZATION;

-- ---------------------------------------------------------------------------
-- 4. CA assessment, device claim and settle.
-- ---------------------------------------------------------------------------
SET SESSION AUTHORIZATION os_t_ca;
-- Verbatim from harvest/src/routes/private_operational_ca.rs.
SELECT os_test.ok((
    SELECT current_user=session_user
        AND pg_has_role(current_user,'sequent_os_initialize_ca','USAGE')
        AND has_function_privilege(current_user,'sequent_backend.operational_initialize_get_ca_enrollment(uuid,uuid,uuid,text,uuid,uuid,text)','EXECUTE')
        AND NOT pg_has_role(current_user,'sequent_os_ledger_api_owner','MEMBER')
        AND NOT pg_has_role(current_user,'sequent_os_ledger_data_owner','MEMBER')
        AND NOT pg_has_role(current_user,'sequent_os_initialize_orchestrator','MEMBER')
        AND NOT pg_has_role(current_user,'sequent_os_initialize_device','MEMBER')
        AND NOT pg_has_role(current_user,'sequent_os_device','MEMBER')
        AND NOT has_table_privilege(current_user,'sequent_backend.operational_signing_intent','SELECT, INSERT, UPDATE, DELETE')
        AND NOT has_any_column_privilege(current_user,'sequent_backend.operational_signing_intent','SELECT, INSERT, UPDATE, REFERENCES')
        AND NOT has_table_privilege(current_user,'sequent_backend.certificate_authority','SELECT, INSERT, UPDATE, DELETE')
        AND NOT has_any_column_privilege(current_user,'sequent_backend.certificate_authority','SELECT, INSERT, UPDATE, REFERENCES')
        AND NOT has_table_privilege(current_user,'sequent_backend.staff_crl','SELECT, INSERT, UPDATE, DELETE')
        AND NOT has_any_column_privilege(current_user,'sequent_backend.staff_crl','SELECT, INSERT, UPDATE, REFERENCES')
        AND NOT has_table_privilege(current_user,'sequent_backend.staff_certificate','SELECT, INSERT, UPDATE, DELETE')
        AND NOT has_any_column_privilege(current_user,'sequent_backend.staff_certificate','SELECT, INSERT, UPDATE, REFERENCES')),
    'Harvest private-CA start-up check holds');
SELECT os_test.ok(
    (SELECT count(*) = 1 AND bool_and(
        binding->>'challenge_id' = :'t2' AND binding->>'action' = 'initialize-voting'
        AND binding->>'key_id' = 'official/' || :'tenant_a' || '/' || :'official'
        AND certificate_id = :'cert1'::uuid AND chain_pem = ARRAY['TEST-LEAF-PEM', 'TEST-ANCHOR-PEM']
        AND grant_until > clock_timestamp() AND ca_version ~ '^[0-9a-f]{64}$'
        AND anchors_pem = ARRAY['TEST-ANCHOR-PEM']
        AND jsonb_array_length(revocation_lists) = 1
        AND revocation_lists->0->>'url' = 'http://crl.invalid/staff.crl'
        AND revocation_lists->0->>'der_hex' = '3000'
        AND (revocation_lists->0->>'fetched_at')::timestamptz < clock_timestamp()
        AND NOT locally_revoked)
     FROM sequent_backend.operational_initialize_get_ca_enrollment(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', 'yubihsm')),
    'CA service reads the reserved enrollment, issuers and revocation lists');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_get_ca_enrollment(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', 'softhsm-test'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_get_ca_enrollment(:'tenant_b', :'event_a', :'req1', :'official', :'t2', :'fence', 'yubihsm'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_get_ca_enrollment(:'tenant_a', :'event_a', :'req1', :'officer', :'t2', :'fence', 'yubihsm'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_get_ca_enrollment(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'t2', 'yubihsm')),
    'no CA enrollment for a wrong profile, tenant, actor or fence');
SELECT sequent_backend.operational_initialize_ca_version(:'tenant_a', :'event_a', :'fingerprint', :'spki') AS ca_v1 \gset
SELECT os_test.ok(
    :'ca_v1' = (SELECT ca_version FROM sequent_backend.operational_initialize_get_ca_enrollment(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', 'yubihsm')),
    'CA version is stable between the two reads');
SELECT os_test.denied(format(
    'SELECT * FROM sequent_backend.operational_signing_get_device_grant(%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence'), 'CA service cannot read the device grant');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_initialize_claim_current_device(%L,%L,%L,%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', 'yubihsm', :'ca_v1'),
    'CA service cannot claim the device');
SELECT os_test.denied('SELECT pem FROM sequent_backend.certificate_authority', 'CA service cannot read issuer tables directly');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(
    sequent_backend.operational_signing_recorded_device_profile(:'tenant_a', :'event_a', :'req1', :'official', :'fence') IS NULL,
    'no recorded device profile before the device claim');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_device;
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_signing_get_device_grant(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence'))
    AND NOT sequent_backend.operational_signing_claim_device_from_reference(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2'),
    'the generic device cannot read or claim an Initialize grant');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_initialize_claim_current_device(%L,%L,%L,%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', 'yubihsm', :'ca_v1'),
    'the generic device cannot use the Initialize claim');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_init_device;
SELECT os_test.ok(
    (SELECT count(*) = 1 AND bool_and(
        binding->>'binding_sha256' = :'h2' AND binding->>'challenge_id' = :'t2'
        AND verified_auth_time = :auth_time AND verified_issuer = :'issuer'
        AND reservation_deadline > clock_timestamp()
        AND reservation_deadline <= (binding->>'intent_expires_at')::timestamptz)
     FROM sequent_backend.operational_signing_get_device_grant(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence')),
    'Initialize device reads its exact grant');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_signing_get_device_grant(:'tenant_b', :'event_a', :'req1', :'official', :'t2', :'fence'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_signing_get_device_grant(:'tenant_a', :'event_a', :'req1', :'officer', :'t2', :'fence'))
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_signing_get_device_grant(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'t2')),
    'no grant for another tenant, actor or fence');
SELECT os_test.ok(
    NOT sequent_backend.operational_initialize_claim_current_device(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', 'yubihsm', :'h2')
    AND NOT sequent_backend.operational_initialize_claim_current_device(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', 'softhsm-test', :'ca_v1')
    AND NOT sequent_backend.operational_initialize_claim_current_device(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h1', 'yubihsm', :'ca_v1')
    AND NOT sequent_backend.operational_initialize_claim_current_device(:'tenant_b', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', 'yubihsm', :'ca_v1'),
    'claim refused for a wrong CA version, profile, binding or tenant');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_signing_claim_device_from_reference(%L,%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2'), 'Initialize device cannot use the generic claim');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_initialize_reserve_own_callback(%L,%L,%L,%L,%L,%L,%L,%L,%L,now(),%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'t2', :'h2', :'n2', :'issuer', 1, 'yubihsm'),
    'Initialize device cannot reserve');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_initialize_ca_version(%L,%L,%L,%L)', :'tenant_a', :'event_a', :'fingerprint', :'spki'),
    'Initialize device cannot compute the CA version itself');
RESET SESSION AUTHORIZATION;

-- The stored revocation list is refreshed between assessment and claim.
UPDATE sequent_backend.staff_crl SET last_ok_at = clock_timestamp()
WHERE tenant_id = :'tenant_a' AND election_event_id = :'event_a';

SET SESSION AUTHORIZATION os_t_init_device;
SELECT os_test.ok(
    NOT sequent_backend.operational_initialize_claim_current_device(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', 'yubihsm', :'ca_v1'),
    'claim refused when the CA assessment version is no longer current');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_ca;
SELECT sequent_backend.operational_initialize_ca_version(:'tenant_a', :'event_a', :'fingerprint', :'spki') AS ca_v2 \gset
SELECT os_test.ok(:'ca_v2' <> :'ca_v1', 'a refreshed revocation list changes the CA version');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_init_device;
SELECT os_test.ok(
    sequent_backend.operational_initialize_claim_current_device(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', 'yubihsm', :'ca_v2'),
    'Initialize device claims the device use');
SELECT os_test.ok(
    NOT sequent_backend.operational_initialize_claim_current_device(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', 'yubihsm', :'ca_v2')
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_signing_get_device_grant(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence')),
    'the device claim is single use and the grant is no longer offered');
SELECT os_test.ok(
    NOT sequent_backend.operational_signing_settle_device_from_reference(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h1', '\x3006020101020101')
    AND NOT sequent_backend.operational_signing_settle_device_from_reference(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', '\x0102030405060708')
    AND NOT sequent_backend.operational_signing_settle_device_from_reference(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', '\x3007020101020101')
    AND NOT sequent_backend.operational_signing_settle_device_from_reference(:'tenant_b', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', '\x3006020101020101'),
    'settle refused for a wrong binding, a non-DER receipt or another tenant');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_device;
SELECT os_test.ok(
    NOT sequent_backend.operational_signing_settle_device_from_reference(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', '\x3006020101020101'),
    'the generic device cannot settle an Initialize claim');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_init_device;
SELECT os_test.ok(
    sequent_backend.operational_signing_settle_device_from_reference(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', '\x3006020101020101'),
    'Initialize device settles its receipt');
SELECT os_test.ok(
    NOT sequent_backend.operational_signing_settle_device_from_reference(:'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', '\x3006020101020101'),
    'the receipt is settled once');
RESET SESSION AUTHORIZATION;

SELECT os_test.ok(
    (SELECT (t.step, t.reservation_id) = ('settled', :'fence'::uuid) FROM sequent_backend.operational_initialize_attempt t WHERE t.challenge_id = :'t2')
    AND (SELECT (t.step, t.return_path) = ('discarded', 'election-show') FROM sequent_backend.operational_initialize_attempt t WHERE t.challenge_id = :'t1')
    AND (SELECT (g.flow, g.device_profile, g.ca_version) = ('initialize', 'yubihsm', :'ca_v2') AND g.settled_at IS NOT NULL
         FROM sequent_backend.operational_signing_device_grant g WHERE g.reservation_id = :'fence'),
    'the typed continuation ended settled; the cancelled attempt stays on record');
SELECT os_test.refused(format(
    'UPDATE sequent_backend.operational_initialize_attempt SET step = %L WHERE challenge_id = %L', 'issued', :'t2'),
    'the continuation cannot move backwards, even for the table owner');
SELECT os_test.refused(format(
    'UPDATE sequent_backend.operational_signing_device_grant SET device_profile = %L WHERE reservation_id = %L', 'softhsm-test', :'fence'),
    'a recorded device profile cannot be rewritten');

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(
    sequent_backend.operational_signing_recorded_device_profile(:'tenant_a', :'event_a', :'req1', :'official', :'fence') = 'yubihsm'
    AND sequent_backend.operational_signing_recorded_device_profile(:'tenant_b', :'event_a', :'req1', :'official', :'fence') IS NULL
    AND sequent_backend.operational_signing_recorded_device_profile(:'tenant_a', :'event_a', :'req1', :'officer', :'fence') IS NULL,
    'approval caller reads the recorded device profile of the exact reservation only');
SELECT os_test.ok(
    (SELECT (phase, challenge, callback_claimed) = ('completed', :'t2'::uuid, true)
     FROM sequent_backend.operational_initialize_current_state(:'tenant_a', :'event_a', :'req1', :'official', :'pay1')),
    'approval caller sees the completed attempt');
-- The two entry points of 0700 the approval transaction uses.
SELECT os_test.ok(
    (SELECT count(*) = 1 AND bool_and(phase = 'completed' AND reservation_id = :'fence'::uuid
        AND receipt = '\x3006020101020101' AND certificate_pem = 'TEST-LEAF-PEM'
        AND binding->>'challenge_id' = :'t2')
     FROM sequent_backend.operational_signing_get_completed(:'tenant_a', :'event_a', :'req1', :'official', :'t2')),
    'approval caller reads the completed receipt without ledger DML');
SELECT os_test.ok(
    NOT sequent_backend.operational_signing_mark_applied_from_evidence(:'tenant_a', :'event_a', :'req1', :'official', :'t2',
        :'fence', '\x3006020101020101', '\x3006020101020101'),
    'marking applied runs for the approval caller and refuses without a recorded approval');
SELECT os_test.ok(
    (SELECT count(*) = 1 FROM sequent_backend.operational_signing_get_official_chain(:'tenant_a', :'event_a', :'official',
        'official/' || :'tenant_a' || '/' || :'official', 1, :'fingerprint', :'spki')),
    'approval caller reads the official chain');
SELECT os_test.ok(
    (SELECT count(*) = 1 FROM sequent_backend.operational_signing_intent p
     WHERE (p.tenant_id, p.event_id, p.request_id, p.actor) = (:'tenant_a', :'event_a', :'req1', :'official')
       AND p.phase = 'completed' AND p.purpose = 'approval-payload'
       AND sequent_backend.operational_signing_recorded_device_profile(p.tenant_id, p.event_id, p.request_id, p.actor, p.reservation_id) = 'yubihsm'
       AND (p.binding).payload_hash = :'pay1' AND p.receipt = '\x3006020101020101'),
    'approval caller reads the evidence columns of the intent ledger');
-- Exact query compiled into Windmill. Exercise it as the real approval caller.
PREPARE quorum(uuid, uuid, uuid, text, text, text, text, text, bytea, text, text, bytea, bytea, text, uuid) AS
SELECT * FROM (
:quorum_sql
) checked(matching);
EXECUTE quorum(:'tenant_a', :'event_a', :'req1', :'official', 'initialize-voting', :'pay1', :'fingerprint', :'spki',
               '\x3006020101020101', NULL, NULL, NULL, NULL, 'yubihsm', :'cert1') \gset
SELECT os_test.ok(NOT :'matching'::boolean, 'a completed receipt is not yet an applied quorum approval');
-- Roll back our synthetic approval, leaving later expiry/revocation tests untouched.
BEGIN;
RESET SESSION AUTHORIZATION;
INSERT INTO sequent_backend.signing_approval
    (tenant_id, election_event_id, request_id, user_id, username, certificate_id, certificate_pem, chain_pem,
     fingerprint_sha256, spki_sha256, holder_sha256, algorithm, payload_signature, revocation_status)
SELECT tenant_id, election_event_id, :'req1', user_id, username, id, pem, 'TEST-CHAIN-PEM',
       fingerprint_sha256, spki_sha256, holder_sha256, 'ecdsa-p256-sha256', '\x3006020101020101', 'checked'
FROM sequent_backend.staff_certificate
WHERE (tenant_id, election_event_id, id, user_id) = (:'tenant_a', :'event_a', :'cert1', :'official');
SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_signing_mark_applied_from_evidence(
    :'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', '\x3006020101020101', '\x3006020101020101'),
    'real approval caller applies the matching receipt and approval together');
EXECUTE quorum(:'tenant_a', :'event_a', :'req1', :'official', 'initialize-voting', :'pay1', :'fingerprint', :'spki',
               '\x3006020101020101', NULL, NULL, NULL, NULL, 'yubihsm', :'cert1') \gset
SELECT os_test.ok(:'matching'::boolean, 'exact applied approval is accepted by the installed provider query');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os14990_windmill;
EXECUTE quorum(:'tenant_a', :'event_a', :'req1', :'official', 'initialize-voting', :'pay1', :'fingerprint', :'spki',
               '\x3006020101020101', NULL, NULL, NULL, NULL, 'yubihsm', :'cert1') \gset
SELECT os_test.ok(:'matching'::boolean, 'restricted Windmill consumer counts the exact applied approval');
EXECUTE quorum(:'tenant_a', :'event_a', :'req1', :'official', 'initialize-voting', :'pay1', :'fingerprint', :'spki',
               '\x3006020101020101', NULL, NULL, NULL, NULL, 'softhsm-test', :'cert1') \gset
SELECT os_test.ok(NOT :'matching'::boolean, 'an applied receipt for another installed provider is refused');
EXECUTE quorum(:'tenant_a', :'event_a', :'req1', :'official', 'initialize-voting', :'pay1', :'fingerprint', :'spki',
               '\x3006020101020102', NULL, NULL, NULL, NULL, 'yubihsm', :'cert1') \gset
SELECT os_test.ok(NOT :'matching'::boolean, 'a changed approval signature is refused');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_harvest;
-- Operator revocation must invalidate a previously applied row before the next
-- official can count it or the execution can release. Nothing is re-signed.
RESET SESSION AUTHORIZATION;
SELECT os_test.ok(sequent_backend.operational_signing_revoke_official(:'tenant_a', :'official',
    (SELECT id FROM sequent_backend.operational_signing_official_enrollment
      WHERE tenant_id=:'tenant_a' AND actor=:'official' AND revoked_at IS NULL),
    'official/' || :'tenant_a' || '/' || :'official', 1), 'operator revoked the previous key');
SET SESSION AUTHORIZATION os_t_harvest;
EXECUTE quorum(:'tenant_a', :'event_a', :'req1', :'official', 'initialize-voting', :'pay1', :'fingerprint', :'spki',
               '\x3006020101020101', NULL, NULL, NULL, NULL, 'yubihsm', :'cert1') \gset
SELECT os_test.ok(NOT :'matching'::boolean, 'a revoked enrollment cannot count its old applied approval');
ROLLBACK;
DEALLOCATE quorum;
SELECT os_test.denied('SELECT * FROM sequent_backend.operational_signing_intent', 'approval caller cannot read the whole intent row');
SELECT os_test.denied('SELECT invoked_at FROM sequent_backend.operational_signing_intent', 'approval caller cannot read other intent columns');
SELECT os_test.denied(format('UPDATE sequent_backend.operational_signing_intent SET phase = %L', 'applied'), 'approval caller cannot write the intent ledger');
SELECT os_test.denied('SELECT * FROM sequent_backend.operational_signing_device_grant', 'approval caller cannot read device grants');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_signing_settle_device_from_reference(%L,%L,%L,%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'req1', :'official', :'t2', :'fence', :'h2', '\x3006020101020101'),
    'approval caller cannot settle a device receipt');
RESET SESSION AUTHORIZATION;

-- ---------------------------------------------------------------------------
-- 5. Expiry: reservation deadline and challenge lifetime.
-- ---------------------------------------------------------------------------
SET SESSION AUTHORIZATION os_t_broker;
SELECT os_test.ok(
    sequent_backend.operational_initialize_prepare(:'tenant_a', :'event_a', :'req2', :'official', :'pay2', 'yubihsm', :'t4', :'issuer', :'n4', :'h4')
    AND sequent_backend.operational_initialize_take_callback(:'tenant_a', :'event_a', :'req2', :'official', :'t4', :'h4'),
    'second request: challenge created and callback redeemed');
RESET SESSION AUTHORIZATION;
SELECT pg_sleep(1.1);
SET SESSION AUTHORIZATION os_t_broker;
SELECT sequent_backend.operational_initialize_reserve_own_callback(:'tenant_a', :'event_a', :'req2', :'official', :'t4', :'h4', :'n4',
    :'issuer', floor(extract(epoch FROM clock_timestamp()))::bigint, clock_timestamp() + interval '1500 milliseconds', 'yubihsm') AS fence4 \gset
SELECT os_test.ok(:'fence4' IS NOT NULL, 'second request: reserved with a proof that expires in 1.5 s');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_ca;
SELECT sequent_backend.operational_initialize_ca_version(:'tenant_a', :'event_a', :'fingerprint', :'spki') AS ca_v4 \gset
RESET SESSION AUTHORIZATION;
SELECT pg_sleep(1.7);
SET SESSION AUTHORIZATION os_t_init_device;
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_signing_get_device_grant(:'tenant_a', :'event_a', :'req2', :'official', :'t4', :'fence4'))
    AND NOT sequent_backend.operational_initialize_claim_current_device(:'tenant_a', :'event_a', :'req2', :'official', :'t4', :'fence4', :'h4', 'yubihsm', :'ca_v4'),
    'an expired reservation is neither offered nor claimable');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_ca;
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_get_ca_enrollment(:'tenant_a', :'event_a', :'req2', :'official', :'t4', :'fence4', 'yubihsm')),
    'no CA enrollment for an expired reservation');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(
    sequent_backend.operational_signing_recorded_device_profile(:'tenant_a', :'event_a', :'req2', :'official', :'fence4') IS NULL
    AND (SELECT phase = 'reserved' FROM sequent_backend.operational_initialize_current_state(:'tenant_a', :'event_a', :'req2', :'official', :'pay2'))
    AND NOT sequent_backend.operational_initialize_discard_unredeemed(:'tenant_a', :'event_a', :'req2', :'official', :'pay2', :'t4'),
    'an unclaimed expired reservation has no device profile and cannot be cancelled');
RESET SESSION AUTHORIZATION;

-- A request that closes in three seconds bounds the challenge lifetime.
SELECT os_test.request(:'req3', :'tenant_a', :'event_a', :'election_a', 'scope-3',
    '{"initialize":"a-3"}', clock_timestamp() + interval '3 seconds');
SELECT payload_sha256 AS pay3 FROM sequent_backend.signing_request WHERE id = :'req3' \gset
SET SESSION AUTHORIZATION os_t_broker;
SELECT os_test.ok(
    sequent_backend.operational_initialize_prepare(:'tenant_a', :'event_a', :'req3', :'official', :'pay3', 'yubihsm', :'t3', :'issuer', :'n3', :'h3'),
    'short-lived challenge created');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(
    (SELECT (phase, challenge, callback_claimed) = ('prepared', :'t3'::uuid, false)
     FROM sequent_backend.operational_initialize_current_state(:'tenant_a', :'event_a', :'req3', :'official', :'pay3')),
    'short-lived challenge is live');
RESET SESSION AUTHORIZATION;
SELECT pg_sleep(3.2);
SET SESSION AUTHORIZATION os_t_broker;
SELECT os_test.ok(
    NOT sequent_backend.operational_initialize_take_callback(:'tenant_a', :'event_a', :'req3', :'official', :'t3', :'h3')
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_context(:'tenant_a', :'event_a', :'req3', :'official', :'pay3', 'yubihsm')),
    'an expired challenge cannot be redeemed and its closed request offers no context');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_current_state(:'tenant_a', :'event_a', :'req3', :'official', :'pay3'))
    AND NOT sequent_backend.operational_initialize_record_browser_return(:'tenant_a', :'event_a', :'req3', :'official', :'t3', 'event-show', 'publish', NULL),
    'an expired challenge is not a live attempt');
SELECT os_test.ok(
    sequent_backend.operational_initialize_discard_unredeemed(:'tenant_a', :'event_a', :'req3', :'official', :'pay3', :'t3'),
    'an expired unredeemed challenge can still be discarded');
RESET SESSION AUTHORIZATION;

-- ---------------------------------------------------------------------------
-- 6. Generic device flow. No function of this contract creates a generic
--    grant; a synthetic one stands in for a future generic orchestrator.
-- ---------------------------------------------------------------------------
\set t5 'a0000000-0000-4000-8000-000000000105'
\set req5 'a0000000-0000-4000-8000-0000000000f5'
SELECT os_test.request(:'req5', :'tenant_a', :'event_a', :'election_a', 'scope-5',
    '{"initialize":"a-5"}', clock_timestamp() + interval '1 hour');
SELECT payload_sha256 AS pay5 FROM sequent_backend.signing_request WHERE id = :'req5' \gset
BEGIN;
SELECT sequent_backend.operational_signing_prepare(ROW(
    :'tenant_a', :'event_a', :'req5', :'official', 'initialize-voting', 'approval-payload',
    'official/' || :'tenant_a' || '/' || :'official', 1, :'fingerprint', :'spki', :'pay5', :'pay5', :'pay5',
    repeat('b5', 32), NULL, NULL, NULL, :'issuer', repeat('05', 32), :'t5', transaction_timestamp(),
    (SELECT expires_at FROM sequent_backend.signing_request WHERE id = :'req5'),
    transaction_timestamp() + interval '4 minutes')::sequent_backend.operational_signing_binding) AS generic_prepared \gset
COMMIT;
SELECT pg_sleep(1.1);
SELECT sequent_backend.operational_signing_reserve(i.binding, floor(extract(epoch FROM clock_timestamp()))::bigint,
    clock_timestamp() + interval '60 seconds') AS fence5
FROM sequent_backend.operational_signing_intent i WHERE i.challenge_id = :'t5' \gset
INSERT INTO sequent_backend.operational_signing_device_grant
    (reservation_id, challenge_id, tenant_id, event_id, request_id, actor, flow, device_profile,
     verified_issuer, verified_auth_time, proof_expires_at)
VALUES (:'fence5', :'t5', :'tenant_a', :'event_a', :'req5', :'official', 'generic', 'yubihsm',
        :'issuer', floor(extract(epoch FROM clock_timestamp()))::bigint, clock_timestamp() + interval '60 seconds');
SELECT os_test.ok(:'generic_prepared'::boolean AND :'fence5' IS NOT NULL, 'synthetic generic reservation exists');

SET SESSION AUTHORIZATION os_t_init_device;
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_signing_get_device_grant(:'tenant_a', :'event_a', :'req5', :'official', :'t5', :'fence5'))
    AND NOT sequent_backend.operational_initialize_claim_current_device(:'tenant_a', :'event_a', :'req5', :'official', :'t5', :'fence5', repeat('b5', 32), 'yubihsm', :'ca_v4'),
    'the Initialize device cannot read or claim a generic grant');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_device;
SELECT os_test.ok(
    (SELECT count(*) = 1 FROM sequent_backend.operational_signing_get_device_grant(:'tenant_a', :'event_a', :'req5', :'official', :'t5', :'fence5')),
    'generic device reads its grant');
SELECT os_test.ok(
    NOT sequent_backend.operational_signing_claim_device_from_reference(:'tenant_a', :'event_a', :'req5', :'official', :'t5', :'fence5', repeat('b6', 32))
    AND NOT sequent_backend.operational_signing_claim_device_from_reference(:'tenant_b', :'event_a', :'req5', :'official', :'t5', :'fence5', repeat('b5', 32)),
    'generic claim refused for a wrong binding or tenant');
SELECT os_test.ok(
    sequent_backend.operational_signing_claim_device_from_reference(:'tenant_a', :'event_a', :'req5', :'official', :'t5', :'fence5', repeat('b5', 32)),
    'generic device claims its grant');
SELECT os_test.ok(
    NOT sequent_backend.operational_signing_claim_device_from_reference(:'tenant_a', :'event_a', :'req5', :'official', :'t5', :'fence5', repeat('b5', 32)),
    'the generic claim is single use');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_init_device;
SELECT os_test.ok(
    NOT sequent_backend.operational_signing_settle_device_from_reference(:'tenant_a', :'event_a', :'req5', :'official', :'t5', :'fence5', repeat('b5', 32), '\x3006020101020101'),
    'the Initialize device cannot settle a generic claim');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_device;
SELECT os_test.ok(
    sequent_backend.operational_signing_settle_device_from_reference(:'tenant_a', :'event_a', :'req5', :'official', :'t5', :'fence5', repeat('b5', 32), '\x3006020101020101')
    AND NOT sequent_backend.operational_signing_settle_device_from_reference(:'tenant_a', :'event_a', :'req5', :'official', :'t5', :'fence5', repeat('b5', 32), '\x3006020101020101'),
    'generic device settles its receipt once');
RESET SESSION AUTHORIZATION;

-- A revoked registration of the key stops every further Initialize step.
UPDATE sequent_backend.staff_certificate
SET status = 'revoked', revoked_at = clock_timestamp(), revoked_by = :'officer', revoke_reason = 'test'
WHERE id = :'cert1';
SET SESSION AUTHORIZATION os_t_broker;
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_context(:'tenant_a', :'event_a', :'req5', :'official', :'pay5', 'yubihsm')),
    'no context once the certificate is revoked');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(
    sequent_backend.operational_official_setup_status(:'tenant_a', :'event_a', :'official') = 'certificate-needs-review',
    'the official sees that the certificate needs review');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_setup;
SELECT os_test.ok(
    (SELECT phase = 'certificate-needs-review' FROM sequent_backend.operational_setup_management_status(:'tenant_a', :'event_a', :'official', :'officer')),
    'the setup authority sees that the certificate needs review');
RESET SESSION AUTHORIZATION;

-- ---------------------------------------------------------------------------
-- 7. MIRU registry.
-- ---------------------------------------------------------------------------
\set rev1 'a0000000-0000-4000-8000-000000000201'
\set eml1 'a0000000-0000-4000-8000-000000000202'
\set xz1 'a0000000-0000-4000-8000-000000000203'
\set csv1 'a0000000-0000-4000-8000-000000000204'
\set rev2 'a0000000-0000-4000-8000-000000000205'
\set old1 'a0000000-0000-4000-8000-000000000206'
\set free1 'a0000000-0000-4000-8000-000000000207'
SELECT repeat('a1', 32) AS sha_a, repeat('a2', 32) AS sha_e, repeat('a3', 32) AS sha_x,
       repeat('a4', 32) AS sha_c, repeat('a5', 32) AS sha_s, repeat('a6', 32) AS sha_o \gset

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(
    sequent_backend.operational_miru_generic_withhold(:'tenant_a', :'event_a', :'free1') IS NULL
    AND sequent_backend.operational_miru_export_exclusions(:'tenant_a', :'event_a') IS NULL,
    'nothing is released or exported before the event inventory is verified');
SELECT os_test.ok(
    sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev1', :'rev1', 'archive', :'sha_a', NULL)
    AND sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev1', :'eml1', 'eml', :'sha_e', NULL)
    AND sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev1', :'xz1', 'xz', :'sha_x', NULL)
    AND sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev1', :'csv1', 'csv', :'sha_c', NULL),
    'runtime registers the four objects of a provisional package revision');
SELECT os_test.ok(
    sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev1', :'eml1', 'eml', :'sha_e', NULL),
    'registering the exact same tuple again is idempotent');
SELECT os_test.ok(
    NOT sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev1', :'eml1', 'eml', :'sha_x', NULL)
    AND NOT sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev1', :'eml1', 'xz', :'sha_e', NULL)
    AND NOT sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev1', :'free1', 'eml', :'sha_o', NULL)
    AND NOT sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev1', :'free1', 'archive', :'sha_o', NULL)
    AND NOT sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev2', :'rev2', 'zip', :'sha_s', NULL)
    AND NOT sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev2', :'rev2', 'archive', 'ABC', NULL),
    'a document cannot be rebound to other content, kind or slot; malformed input is refused');
SELECT os_test.ok(
    NOT sequent_backend.operational_miru_register_artifact(:'tenant_b', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev2', :'rev2', 'archive', :'sha_s', NULL)
    AND NOT sequent_backend.operational_miru_register_artifact(:'tenant_b', :'event_b', :'tally_a', :'election_a', :'area_a', :'rev2', :'rev2', 'archive', :'sha_s', NULL)
    AND NOT sequent_backend.operational_miru_register_artifact(:'tenant_l', :'event_l', :'tally_a', :'election_a', :'area_a', :'rev2', :'rev2', 'archive', :'sha_s', NULL),
    'registration refused across tenants and for a legacy tenant');
SELECT os_test.ok(
    sequent_backend.operational_miru_generic_withhold(:'tenant_a', :'event_a', :'eml1')
    AND sequent_backend.operational_miru_generic_withhold(:'tenant_a', NULL, :'rev1')
    AND sequent_backend.operational_miru_generic_withhold(:'tenant_a', :'event_a', :'free1') IS NULL,
    'a registered document is withheld at once; anything else stays undecided');
SELECT os_test.denied(format(
    'INSERT INTO sequent_backend.operational_miru_inventory (tenant_id, event_id) VALUES (%L, %L)', :'tenant_a', :'event_a'),
    'runtime cannot attest the inventory');
SELECT os_test.denied(format(
    'INSERT INTO sequent_backend.operational_miru_artifact (tenant_id, event_id, document_id, kind, content_sha256) VALUES (%L,%L,%L,%L,%L)',
    :'tenant_a', :'event_a', :'free1', 'eml', :'sha_o'), 'runtime cannot write the registry directly');
SELECT os_test.denied('DELETE FROM sequent_backend.operational_miru_artifact', 'runtime cannot delete from the registry');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_admin;
SELECT os_test.refused(format(
    'INSERT INTO sequent_backend.operational_miru_inventory (tenant_id, event_id) VALUES (%L, %L)', :'tenant_l', :'event_l'),
    'no inventory for a legacy tenant');
SELECT os_test.refused(format(
    'INSERT INTO sequent_backend.operational_miru_inventory (tenant_id, event_id, state, verified_at) VALUES (%L, %L, %L, now())',
    :'tenant_a', :'event_a', 'verified'), 'an inventory cannot be created already verified');
INSERT INTO sequent_backend.operational_miru_inventory (tenant_id, event_id)
VALUES (:'tenant_a', :'event_a') RETURNING inventory_id AS inventory \gset
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(
    NOT sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'old1', :'old1', 'archive', :'sha_o', :'inventory')
    AND sequent_backend.operational_miru_generic_withhold(:'tenant_a', :'event_a', :'free1') IS NULL,
    'runtime cannot register under an inventory; an open inventory decides nothing');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_admin;
SELECT os_test.ok(
    NOT sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'old1', :'old1', 'archive', :'sha_o', :'tally_a')
    AND sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'old1', :'old1', 'archive', :'sha_o', :'inventory'),
    'the deployment admin classifies a pre-existing document under the open inventory');
UPDATE sequent_backend.operational_miru_inventory SET state = 'verified', verified_at = clock_timestamp()
WHERE (tenant_id, event_id) = (:'tenant_a', :'event_a');
SELECT os_test.refused(format(
    'UPDATE sequent_backend.operational_miru_inventory SET state = %L, verified_at = NULL WHERE tenant_id = %L', 'open', :'tenant_a'),
    'a verified inventory cannot be reopened');
SELECT os_test.ok(
    NOT sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev2', :'rev2', 'archive', :'sha_s', :'inventory'),
    'nothing is classified under a verified inventory');
SELECT os_test.refused(format(
    'UPDATE sequent_backend.operational_miru_artifact SET content_sha256 = %L WHERE document_id = %L', :'sha_s', :'old1'),
    'registry rows are immutable, even for the table owner');
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_miru_generic_withhold(%L,%L,%L)', :'tenant_a', :'event_a', :'free1'),
    'the deployment admin is not a runtime caller');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(
    sequent_backend.operational_miru_generic_withhold(:'tenant_a', :'event_a', :'free1') = false
    AND sequent_backend.operational_miru_generic_withhold(:'tenant_a', NULL, :'free1') = false
    AND sequent_backend.operational_miru_generic_withhold(:'tenant_a', :'event_a', :'csv1')
    AND sequent_backend.operational_miru_generic_withhold(:'tenant_a', :'event_a', :'old1'),
    'after verification ordinary documents are released and registered ones stay withheld');
SELECT os_test.ok(
    sequent_backend.operational_miru_generic_withhold(:'tenant_b', :'event_b', :'free1') IS NULL
    AND sequent_backend.operational_miru_generic_withhold(:'tenant_l', :'event_l', :'free1') IS NULL
    AND sequent_backend.operational_miru_export_exclusions(:'tenant_b', :'event_b') IS NULL
    AND sequent_backend.operational_miru_export_exclusions(:'tenant_b', :'event_a') IS NULL
    AND sequent_backend.operational_miru_export_exclusions(:'tenant_l', :'event_l') IS NULL,
    'another tenant''s event and a legacy tenant stay undecided');
SELECT os_test.ok(
    (SELECT e @> ARRAY[:'rev1', :'eml1', :'xz1', :'csv1', :'old1']::uuid[] AND cardinality(e) = 5
     FROM sequent_backend.operational_miru_export_exclusions(:'tenant_a', :'event_a') AS e),
    'the export exclusions are exactly the registered documents');
-- The signed revision shares the EML and XZ documents of the provisional one.
SELECT os_test.ok(
    sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev2', :'rev2', 'archive', :'sha_s', NULL)
    AND sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev2', :'eml1', 'eml', :'sha_e', NULL)
    AND sequent_backend.operational_miru_register_artifact(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev2', :'xz1', 'xz', :'sha_x', NULL),
    'runtime registers the signed revision over the same EML and XZ');
-- Verbatim statement of windmill/src/services/signing/actions/transmission.rs.
PREPARE custody(uuid, uuid, uuid, uuid, uuid, uuid, text, uuid, text, uuid, text) AS
SELECT EXISTS (
    SELECT FROM sequent_backend.operational_miru_revision_artifact first
    JOIN sequent_backend.operational_miru_revision_artifact second
      USING (tenant_id,event_id,tally_session_id,election_id,area_id,revision_id)
    JOIN sequent_backend.operational_miru_revision_artifact third
      USING (tenant_id,event_id,tally_session_id,election_id,area_id,revision_id)
    JOIN sequent_backend.operational_miru_artifact a
      ON (a.tenant_id,a.event_id,a.document_id)=(first.tenant_id,first.event_id,first.document_id)
    JOIN sequent_backend.operational_miru_artifact e
      ON (e.tenant_id,e.event_id,e.document_id)=(second.tenant_id,second.event_id,second.document_id)
    JOIN sequent_backend.operational_miru_artifact x
      ON (x.tenant_id,x.event_id,x.document_id)=(third.tenant_id,third.event_id,third.document_id)
    WHERE (first.tenant_id,first.event_id,first.tally_session_id,first.election_id,first.area_id)
          = ($1,$2,$3,$4,$5)
      AND first.revision_id=$6
      AND (first.kind,first.document_id,a.content_sha256)=('archive',$6,$7)
      AND (second.kind,second.document_id,e.content_sha256)=('eml',$8,$9)
      AND (third.kind,third.document_id,x.content_sha256)=('xz',$10,$11)
) AS custody;
EXECUTE custody(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev2', :'sha_s', :'eml1', :'sha_e', :'xz1', :'sha_x') \gset
SELECT os_test.ok(:'custody'::boolean, 'the Windmill custody statement proves the signed revision');
EXECUTE custody(:'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev2', :'sha_a', :'eml1', :'sha_e', :'xz1', :'sha_x') \gset
SELECT os_test.ok(NOT :'custody'::boolean, 'the custody statement refuses another archive digest');
EXECUTE custody(:'tenant_b', :'event_a', :'tally_a', :'election_a', :'area_a', :'rev2', :'sha_s', :'eml1', :'sha_e', :'xz1', :'sha_x') \gset
SELECT os_test.ok(NOT :'custody'::boolean, 'the custody statement refuses another tenant');
DEALLOCATE custody;
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_broker;
SELECT os_test.denied(format(
    'SELECT sequent_backend.operational_miru_register_artifact(%L,%L,%L,%L,%L,%L,%L,%L,%L,NULL)',
    :'tenant_a', :'event_a', :'tally_a', :'election_a', :'area_a', :'free1', :'free1', 'archive', :'sha_o'),
    'a physical-side principal cannot register MIRU output');
RESET SESSION AUTHORIZATION;

-- ---------------------------------------------------------------------------
-- 8. A legacy-p12 tenant is unaffected.
-- ---------------------------------------------------------------------------
SELECT os_test.ok(
    (SELECT mode = 'legacy-p12' FROM sequent_backend.operational_signing_policy WHERE singleton)
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_signing_required_tenant WHERE tenant_id = :'tenant_l')
    AND NOT sequent_backend.operational_signing_hsm_required(:'tenant_l')
    AND sequent_backend.operational_signing_hsm_required(:'tenant_a'),
    'the deployment default stays legacy-p12 and the legacy tenant is not promoted');
SELECT os_test.ok(
    NOT EXISTS (
        SELECT FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid
        JOIN pg_proc p ON p.oid = t.tgfoid
        WHERE NOT t.tgisinternal AND c.relnamespace = 'sequent_backend'::regnamespace
          AND p.proowner = 'sequent_os_ledger_api_owner'::regrole
          AND c.relowner <> 'sequent_os_ledger_data_owner'::regrole),
    'the contract adds no trigger to any pre-existing table');
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_attempt WHERE tenant_id = :'tenant_l')
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_signing_device_grant WHERE tenant_id = :'tenant_l')
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_setup_generation WHERE tenant_id = :'tenant_l')
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_miru_artifact WHERE tenant_id = :'tenant_l')
    AND NOT EXISTS (SELECT FROM sequent_backend.operational_signing_intent WHERE tenant_id = :'tenant_l'),
    'no ledger row was ever written for the legacy tenant');
-- Its ordinary signing rows still change and delete as before.
UPDATE sequent_backend.signing_request SET status = 'cancelled', cancel_reason = 'by-requester', cancelled_by = 'requester'
WHERE id = :'reql';
DELETE FROM sequent_backend.signing_request WHERE id = :'reql';
SELECT os_test.ok(
    NOT EXISTS (SELECT FROM sequent_backend.signing_request WHERE id = :'reql')
    AND (SELECT count(*) = 1 FROM sequent_backend.election_event WHERE id = :'event_l'),
    'a legacy tenant''s signing request is updated and deleted as before');
\echo ASSERTIONS-COMPLETE

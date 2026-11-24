-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
\set ON_ERROR_STOP 1
\set t 'a0000000-0000-4000-8000-000000000001'
\set e 'a0000000-0000-4000-8000-000000000002'
\set p 'a0000000-0000-4000-8000-000000000003'
\set r 'a0000000-0000-4000-8000-0000000000f1'
\set r2 'a0000000-0000-4000-8000-0000000000f2'
\set official 'a0000000-0000-4000-8000-0000000000a5'
\set expiry 'a0000000-0000-4000-8000-0000000000a6'
\set officer 'a0000000-0000-4000-8000-0000000000a2'
\set attempt 'a0000000-0000-4000-8000-0000000000aa'
\set expired_attempt 'a0000000-0000-4000-8000-0000000000ab'
\set cert 'a0000000-0000-4000-8000-0000000000ca'
SELECT encode(sha256(convert_to('{"initialize":"a-1"}','UTF8')),'hex') AS hash,
       encode(sha256(convert_to('{"initialize":"a-2"}','UTF8')),'hex') AS hash2,
       repeat('a4',32) AS spki, repeat('b4',32) AS fingerprint, repeat('c4',32) AS evidence,
       E'-----BEGIN CERTIFICATE REQUEST-----\nTUlJQ0FUQ0NBZWtDQVFBd1h6RUxNQWtHQTFVRUJoTUNWVk14\n-----END CERTIFICATE REQUEST-----\n' AS csr
\gset

INSERT INTO sequent_backend.operational_initialize_setup_roster VALUES
    (:'t',:'e',:'official',:'officer',clock_timestamp() + interval '10 minutes'),
    (:'t',:'e',:'expiry',:'officer',clock_timestamp() + interval '10 minutes');

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r',:'p',:'official',:'hash') = 'setup-available', 'fresh exact scope can prepare');
SELECT os_test.ok(sequent_backend.operational_initialize_start_self_setup(
    :'t',:'e',:'r',:'p',:'official',:'hash',clock_timestamp(),:'attempt'),
    'first click reserves before a lost dispatch');
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r',:'p',:'official',:'hash') = 'setup-retry-available',
    'failure before claim allows explicit retry and does not infer completion');
SELECT os_test.ok(sequent_backend.operational_initialize_redispatch_self_setup(
    :'t',:'e',:'r',:'p',:'official',:'hash',clock_timestamp() - interval '61 seconds') IS NULL,
    'stale Gold cannot redeliver');
SELECT os_test.ok(sequent_backend.operational_initialize_redispatch_self_setup(
    :'t',:'e',:'r',:'p',:'official',:'hash',clock_timestamp()) = :'attempt',
    'fresh Gold recovery redelivers the exact saved attempt');
SELECT os_test.ok(sequent_backend.operational_initialize_redispatch_self_setup(
    :'t',:'e',:'r',:'p',:'official',:'hash',clock_timestamp()) = :'attempt',
    'duplicate preclaim delivery can only identify the same attempt');
SELECT os_test.ok(sequent_backend.operational_initialize_redispatch_self_setup(
    :'t',:'e',:'r2',:'p',:'official',:'hash2',clock_timestamp()) IS NULL,
    'another live request on the same Post cannot redeliver');
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r2',:'p',:'official',:'hash2') = 'unavailable',
    'another live request on the same Post cannot read the enrolled attempt phase');
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r',:'official',:'official',:'hash') IS NULL
    AND sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r',:'p',:'official',:'hash2') IS NULL,
    'wrong Post or digest cannot read setup state');
SELECT os_test.ok(sequent_backend.operational_initialize_redispatch_self_setup(
    :'t',:'e',:'r',:'p',:'officer',:'hash',clock_timestamp()) IS NULL,
    'another actor cannot redeliver even with the same exact request');
SELECT os_test.denied('SELECT * FROM sequent_backend.operational_initialize_claim_self_setup(NULL,NULL,NULL,NULL)',
    'approval caller is not the device claim principal');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_setup_device;
SELECT fence FROM sequent_backend.operational_initialize_claim_self_setup(
    :'t',:'e',:'official',:'attempt') \gset
SELECT os_test.ok((SELECT count(*) FROM sequent_backend.operational_initialize_claim_self_setup(
    :'t',:'e',:'official',:'attempt')) = 0, 'in-flight duplicate delivery cannot claim again');
SELECT os_test.denied(
    'SELECT sequent_backend.operational_initialize_redispatch_self_setup(NULL,NULL,NULL,NULL,NULL,NULL,NULL)',
    'physical worker cannot authorize a browser redelivery');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r',:'p',:'official',:'hash') = 'setup-pending',
    'loss after claim is pending, not retryable');
SELECT os_test.ok(sequent_backend.operational_initialize_redispatch_self_setup(
    :'t',:'e',:'r',:'p',:'official',:'hash',clock_timestamp()) IS NULL,
    'fresh Gold cannot reattempt after physical claim');
SELECT os_test.ok(sequent_backend.operational_initialize_start_self_setup(
    :'t',:'e',:'r',:'p',:'expiry',:'hash',clock_timestamp(),:'expired_attempt'),
    'different official reserves a separate unclaimed generation');
RESET SESSION AUTHORIZATION;

-- Simulate elapsed wall time in this disposable DB only. No application or
-- worker role can change claim/deadline history; no sleeps/races in the test.
SET session_replication_role = replica;
UPDATE sequent_backend.operational_setup_generation
SET claimed_at = clock_timestamp() - interval '2 minutes',
    device_deadline = clock_timestamp() - interval '1 minute'
WHERE attempt = :'attempt';
UPDATE sequent_backend.operational_setup_generation
SET prepared_at = clock_timestamp() - interval '3 minutes',
    claim_deadline = clock_timestamp() - interval '1 minute'
WHERE attempt = :'expired_attempt';
SET session_replication_role = DEFAULT;

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r',:'p',:'official',:'hash') = 'setup-recovery-required',
    'claimed operation beyond its deadline requires operator recovery');
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r',:'p',:'expiry',:'hash') = 'setup-recovery-required',
    'unclaimed operation beyond its original deadline requires operator recovery');
SELECT os_test.ok(sequent_backend.operational_initialize_redispatch_self_setup(
    :'t',:'e',:'r',:'p',:'expiry',:'hash',clock_timestamp()) IS NULL,
    'expiry never extends the claim window');
SELECT os_test.ok(NOT sequent_backend.operational_initialize_start_self_setup(
    :'t',:'e',:'r',:'p',:'expiry',:'hash',clock_timestamp(),gen_random_uuid()),
    'expiry never permits replacement generation');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_setup_device;
SELECT os_test.ok(NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_claim_self_setup(
    :'t',:'e',:'expiry',:'expired_attempt')), 'the expired prepared attempt cannot reach the provider');
SELECT sequent_backend.operational_setup_complete_generation(
    :'t',:'e',:'official',:'attempt',:'fence',:'spki',:'evidence',:'csr') AS enrollment \gset
SELECT os_test.ok(:'enrollment' IS NOT NULL, 'already-claimed device can settle late durable completion');
SELECT os_test.ok(NOT EXISTS (SELECT FROM sequent_backend.operational_initialize_claim_self_setup(
    :'t',:'e',:'official',:'attempt')), 'completion still cannot trigger another physical claim');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r',:'p',:'official',:'hash') = 'setup-pending',
    'loss after durable device completion recovers as awaiting officer certificate');
RESET SESSION AUTHORIZATION;

-- Model the already trusted independent issuer/registration. This is a
-- persistence/status contract, not an X.509 or PKCS#11 test.
INSERT INTO sequent_backend.staff_certificate
    (id,tenant_id,election_event_id,election_id,user_id,username,fingerprint_sha256,spki_sha256,
     holder_sha256,serial,subject,issuer,not_before,not_after,pem,registration,registered_by)
VALUES (:'cert',:'t',:'e',:'p',:'official','fixture-official',:'fingerprint',:'spki',
     repeat('d4',32),'02','CN=Fixture Official','CN=Fixture CA',now()-interval '1 hour',
     now()+interval '1 day','FIXTURE-LEAF-PEM','security-officer',:'officer');
SET SESSION AUTHORIZATION os_t_setup;
SELECT os_test.ok(sequent_backend.operational_setup_attach_certificate(
    :'t',:'e',:'official',:'officer',:'attempt',:'enrollment',:'cert',:'fingerprint',ARRAY['FIXTURE-LEAF-PEM']),
    'existing setup authority independently attaches the exact certificate');
RESET SESSION AUTHORIZATION;

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r',:'p',:'official',:'hash') = 'certificate-on-file',
    'loss after certificate attachment is reconciled from the exact durable receipt');
SELECT os_test.ok(NOT sequent_backend.operational_initialize_start_self_setup(
    :'t',:'e',:'r',:'p',:'official',:'hash',clock_timestamp(),gen_random_uuid()),
    'attached certificate never allows a replacement key');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_nobody;
SELECT os_test.denied(
    'SELECT sequent_backend.operational_initialize_scoped_setup_status(NULL,NULL,NULL,NULL,NULL,NULL)',
    'public role cannot read the exact setup status');
RESET SESSION AUTHORIZATION;

SELECT os_test.ok(NOT EXISTS (SELECT FROM sequent_backend.signing_approval
    WHERE (tenant_id,election_event_id,request_id) = (:'t',:'e',:'r')),
    'neither preparation, redelivery, completion nor attachment creates an approval');

-- A later independent request for the same Post can use an existing eligible
-- certificate, but does not gain access to the original attempt.
\set other_post 'a0000000-0000-4000-8000-00000000000d'
\set other_request 'a0000000-0000-4000-8000-0000000000f3'
\set preenrolled 'a0000000-0000-4000-8000-0000000000a7'
\set precert 'a0000000-0000-4000-8000-0000000000cb'
SELECT encode(sha256(convert_to('{"initialize":"new-post"}','UTF8')),'hex') AS other_hash,
       repeat('a5',32) AS prespki, repeat('b5',32) AS prefp \gset
INSERT INTO sequent_backend.election (id,tenant_id,election_event_id)
VALUES (:'other_post',:'t',:'e');
SELECT os_test.request(:'other_request',:'t',:'e',:'other_post','scope-3',
    '{"initialize":"new-post"}',clock_timestamp()+interval '1 hour');

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r2',:'p',:'official',:'hash2') = 'certificate-on-file',
    'valid certificate from a prior request is ready for the same official and Post');
SELECT os_test.ok(sequent_backend.operational_initialize_redispatch_self_setup(
    :'t',:'e',:'r2',:'p',:'official',:'hash2',clock_timestamp()) IS NULL
    AND NOT sequent_backend.operational_initialize_start_self_setup(
    :'t',:'e',:'r2',:'p',:'official',:'hash2',clock_timestamp(),gen_random_uuid()),
    'a later ready request cannot redeliver or create another generation');
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'other_request',:'other_post',:'official',:'other_hash') = 'unavailable',
    'Post-scoped certificate is unavailable to a different actual Post');
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r2',:'p',:'officer',:'hash2') = 'unavailable',
    'another actor cannot observe the original official certificate readiness');
RESET SESSION AUTHORIZATION;

SELECT sequent_backend.operational_signing_enroll_official(
    :'t',:'preenrolled','official/'||:'t'||'/'||:'preenrolled',1,:'prespki','yubihsm',true,repeat('a7',32))
    AS pre_enrollment \gset
INSERT INTO sequent_backend.staff_certificate
    (id,tenant_id,election_event_id,user_id,username,fingerprint_sha256,spki_sha256,
     holder_sha256,serial,subject,issuer,not_before,not_after,pem,registration,registered_by)
VALUES (:'precert',:'t',:'e',:'preenrolled','preenrolled-fixture',:'prefp',:'prespki',
     repeat('d5',32),'03','CN=Pre-enrolled Official','CN=Fixture CA',now()-interval '1 hour',
     now()+interval '1 day','FIXTURE-PRE-LEAF-PEM','security-officer',:'officer');
SELECT os_test.ok(sequent_backend.operational_signing_bind_official_certificate(
    :'t',:'e',:'preenrolled',:'pre_enrollment',:'precert',:'prefp',ARRAY['FIXTURE-PRE-LEAF-PEM']),
    'fixture operator pre-enrolls an independently attached event-wide certificate');

SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'other_request',:'other_post',:'preenrolled',:'other_hash') = 'certificate-on-file',
    'legitimate pre-enrollment with no setup record is ready for a matching event-wide certificate');
RESET SESSION AUTHORIZATION;

-- Trusted fixture mutation simulates stored key/certificate drift. Each
-- check must fail independently; the transaction is never retained.
BEGIN;
UPDATE sequent_backend.staff_certificate SET pem='FIXTURE-DIFFERENT-LEAF' WHERE id=:'cert';
SET LOCAL SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r2',:'p',:'official',:'hash2') = 'setup-recovery-required',
    'mismatched current PEM and attached certificate are not ready');
ROLLBACK;
BEGIN;
UPDATE sequent_backend.operational_signing_official_enrollment SET device_profile='softhsm-test'
    WHERE id=:'enrollment';
SET LOCAL SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r2',:'p',:'official',:'hash2') = 'setup-recovery-required',
    'a certificate for a changed device profile is not ready');
ROLLBACK;
BEGIN;
UPDATE sequent_backend.staff_certificate SET status='revoked',revoked_at=clock_timestamp() WHERE id=:'cert';
SET LOCAL SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r2',:'p',:'official',:'hash2') = 'setup-recovery-required',
    'locally revoked staff certificate is not ready');
ROLLBACK;
BEGIN;
UPDATE sequent_backend.operational_signing_official_enrollment SET revoked_at=clock_timestamp() WHERE id=:'enrollment';
SET LOCAL SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r2',:'p',:'official',:'hash2') = 'setup-recovery-required',
    'revoked device enrollment is not ready');
ROLLBACK;

SELECT os_test.ok(
    (SELECT count(*) FROM sequent_backend.operational_setup_generation WHERE official=:'official') = 1
    AND NOT EXISTS (SELECT FROM sequent_backend.signing_approval WHERE request_id IN (:'r',:'r2')),
    'valid new requests create neither setup generations, nor approvals before explicit signing');

-- The same unclaimed attempt first blocks another live request silently,
-- then becomes operator-only when the original is closed. Do not let the
-- new request locate, take over or mint a replacement for the old attempt.
\set orphan 'a0000000-0000-4000-8000-0000000000a8'
\set orphan_attempt 'a0000000-0000-4000-8000-0000000000ad'
INSERT INTO sequent_backend.operational_initialize_setup_roster VALUES
    (:'t',:'e',:'orphan',:'officer',clock_timestamp() + interval '10 minutes');
SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_start_self_setup(
    :'t',:'e',:'r',:'p',:'orphan',:'hash',clock_timestamp(),:'orphan_attempt'),
    'older request starts an unclaimed enrollment');
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r2',:'p',:'orphan',:'hash2') = 'unavailable',
    'other still-live prepared request is not visible or actionable');
RESET SESSION AUTHORIZATION;
UPDATE sequent_backend.signing_request SET status='expired' WHERE id=:'r';
SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'r2',:'p',:'orphan',:'hash2') = 'setup-recovery-required',
    'expired original request surfaces generic operator recovery for the same Post and official');
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'other_request',:'other_post',:'orphan',:'other_hash') = 'unavailable',
    'orphaned setup is not disclosed to a different Post');
SELECT os_test.ok(sequent_backend.operational_initialize_redispatch_self_setup(
    :'t',:'e',:'r2',:'p',:'orphan',:'hash2',clock_timestamp()) IS NULL
    AND NOT sequent_backend.operational_initialize_start_self_setup(
    :'t',:'e',:'r2',:'p',:'orphan',:'hash2',clock_timestamp(),gen_random_uuid()),
    'generic orphan recovery exposes no retry or replacement');
RESET SESSION AUTHORIZATION;

BEGIN;
UPDATE sequent_backend.operational_signing_official_enrollment SET revoked_at=clock_timestamp()
    WHERE id=:'pre_enrollment';
SET LOCAL SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_scoped_setup_status(
    :'t',:'e',:'other_request',:'other_post',:'preenrolled',:'other_hash') = 'setup-recovery-required',
    'revoked event-wide pre-enrollment needs an operator even without self-setup history');
ROLLBACK;

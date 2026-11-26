-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
\set ON_ERROR_STOP 1
\set tenant 'a0000000-0000-4000-8000-000000000001'
\set event 'a0000000-0000-4000-8000-000000000002'
\set post 'a0000000-0000-4000-8000-000000000003'
\set request 'a0000000-0000-4000-8000-0000000000f1'
\set official 'a0000000-0000-4000-8000-0000000000a1'
\set officer 'a0000000-0000-4000-8000-0000000000a2'
\set other 'a0000000-0000-4000-8000-0000000000a3'
\set attempt 'a0000000-0000-4000-8000-0000000000a9'
SELECT encode(sha256(convert_to('{"initialize":"a-1"}','UTF8')),'hex') AS hash \gset
INSERT INTO sequent_backend.operational_setup_installation (tenant_id,device_profile)
VALUES (:'tenant','yubihsm');
INSERT INTO sequent_backend.operational_initialize_setup_roster VALUES
    (:'tenant',:'event',:'official',:'officer',clock_timestamp() + interval '10 minutes');
SET SESSION AUTHORIZATION os_t_harvest;
SELECT os_test.ok(sequent_backend.operational_initialize_self_setup_status(:'tenant',:'event',:'official') =
    'setup-available', 'only the operator-preauthorized official can begin');
SELECT os_test.ok(sequent_backend.operational_initialize_self_setup_status(:'tenant',:'event',:'other') =
    'unavailable', 'unlisted tenant member cannot set up');
SELECT os_test.ok(NOT sequent_backend.operational_initialize_start_self_setup(
    :'tenant',:'event',:'request',:'post',:'official',:'hash',
    clock_timestamp() - interval '61 seconds',:'attempt'), 'old Gold authentication refused');
SELECT os_test.ok(NOT sequent_backend.operational_initialize_start_self_setup(
    :'tenant',:'event',:'request',:'post',:'official',:'hash',
    clock_timestamp() + interval '1 seconds',:'attempt'), 'future Gold authentication refused');
SELECT os_test.ok(NOT sequent_backend.operational_initialize_start_self_setup(
    :'tenant',:'event',:'request',:'other',:'official',:'hash',
    clock_timestamp(),:'attempt'), 'another Post cannot be substituted');
SELECT os_test.ok(NOT sequent_backend.operational_initialize_start_self_setup(
    :'tenant',:'event',:'request',:'post',:'official',repeat('f',64),
    clock_timestamp(),:'attempt'), 'other canonical intent cannot be substituted');
SELECT os_test.ok(sequent_backend.operational_initialize_start_self_setup(
    :'tenant',:'event',:'request',:'post',:'official',:'hash',
    clock_timestamp(),:'attempt'), 'valid setup records one generation request');
SELECT os_test.ok(NOT sequent_backend.operational_initialize_start_self_setup(
    :'tenant',:'event',:'request',:'post',:'official',:'hash',
    clock_timestamp(),gen_random_uuid()), 'cannot replay or replace one generation');
SELECT os_test.ok(sequent_backend.operational_initialize_self_setup_status(:'tenant',:'event',:'official') =
    'setup-pending', 'await independent CA registration only');
RESET SESSION AUTHORIZATION;
SELECT os_test.ok(NOT EXISTS (SELECT FROM sequent_backend.signing_approval
    WHERE (tenant_id,election_event_id,request_id) = (:'tenant',:'event',:'request')),
    'setup is not an approval');
SET SESSION AUTHORIZATION os_t_setup_device;
SELECT os_test.ok(has_function_privilege(current_user,
    'sequent_backend.operational_setup_claim_generation(uuid,uuid,uuid,uuid)', 'EXECUTE'),
    'ordinary deployment keeps existing officer-managed preparation intact');
SELECT os_test.ok((SELECT count(*) FROM sequent_backend.operational_initialize_claim_self_setup(
    :'tenant',:'event',:'other',:'attempt')) = 0, 'another official cannot claim the prepared device generation');
SELECT os_test.ok((SELECT count(*) FROM sequent_backend.operational_initialize_claim_self_setup(
    :'tenant',:'event',:'official',:'attempt')) = 1, 'existing device role claims its exact generation');
SELECT os_test.ok((SELECT count(*) FROM sequent_backend.operational_initialize_claim_self_setup(
    :'tenant',:'event',:'official',:'attempt')) = 0, 'device cannot claim twice');
RESET SESSION AUTHORIZATION;
SET SESSION AUTHORIZATION os_t_nobody;
SELECT os_test.denied('SELECT sequent_backend.operational_initialize_self_setup_status(NULL,NULL,NULL)',
    'no public execute on scoped setup status');
RESET SESSION AUTHORIZATION;

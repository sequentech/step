-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Test-only logins and synthetic rows. Nothing here is part of a migration.
-- Runs as the cluster superuser of the disposable container.
\set ON_ERROR_STOP 1

CREATE ROLE os_t_harvest LOGIN;
CREATE ROLE os_t_setup LOGIN;
CREATE ROLE os_t_setup_device LOGIN;
CREATE ROLE os_t_broker LOGIN;
CREATE ROLE os_t_ca LOGIN;
CREATE ROLE os_t_init_device LOGIN;
CREATE ROLE os_t_device LOGIN;
CREATE ROLE os_t_admin LOGIN;
CREATE ROLE os_t_nobody LOGIN;
GRANT sequent_os_approve TO os_t_harvest;
GRANT sequent_os_setup_authority TO os_t_setup;
GRANT sequent_os_setup_device TO os_t_setup_device;
GRANT sequent_os_initialize_orchestrator TO os_t_broker;
GRANT sequent_os_initialize_ca TO os_t_ca;
GRANT sequent_os_initialize_device TO os_t_init_device;
GRANT sequent_os_device TO os_t_device;
GRANT sequent_os_ledger_data_owner TO os_t_admin;
-- The ordinary application privilege the verbatim Windmill quorum query needs.
GRANT SELECT ON sequent_backend.signing_document_revision TO os_t_harvest;
GRANT USAGE ON SCHEMA sequent_backend TO os_t_nobody;

-- Assertion helpers, callable by every test login.
CREATE SCHEMA os_test;
GRANT USAGE ON SCHEMA os_test TO PUBLIC;
CREATE FUNCTION os_test.ok(p_condition boolean, p_name text) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
    IF p_condition IS NOT TRUE THEN
        RAISE EXCEPTION 'FAILED as %: %', session_user, p_name;
    END IF;
    RAISE NOTICE 'ok - [%] %', session_user, p_name;
END;
$$;
-- The statement must fail with "permission denied" (SQLSTATE 42501).
CREATE FUNCTION os_test.denied(p_sql text, p_name text) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
    BEGIN
        EXECUTE p_sql;
    EXCEPTION WHEN insufficient_privilege THEN
        RAISE NOTICE 'ok - [%] denied: %', session_user, p_name;
        RETURN;
    END;
    RAISE EXCEPTION 'FAILED as %: not denied: %', session_user, p_name;
END;
$$;
-- The statement must raise an error that is not a permission error.
CREATE FUNCTION os_test.refused(p_sql text, p_name text) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
    BEGIN
        EXECUTE p_sql;
    EXCEPTION
        WHEN insufficient_privilege THEN
            RAISE EXCEPTION 'FAILED as %: permission error instead of refusal: %', session_user, p_name;
        WHEN OTHERS THEN
            RAISE NOTICE 'ok - [%] refused: %', session_user, p_name;
            RETURN;
    END;
    RAISE EXCEPTION 'FAILED as %: not refused: %', session_user, p_name;
END;
$$;

\set tenant_a 'a0000000-0000-4000-8000-000000000001'
\set tenant_b 'b0000000-0000-4000-8000-000000000001'
\set tenant_l 'c0000000-0000-4000-8000-000000000001'
\set event_a 'a0000000-0000-4000-8000-000000000002'
\set event_b 'b0000000-0000-4000-8000-000000000002'
\set event_l 'c0000000-0000-4000-8000-000000000002'
\set election_a 'a0000000-0000-4000-8000-000000000003'
\set election_l 'c0000000-0000-4000-8000-000000000003'
\set area_a 'a0000000-0000-4000-8000-000000000004'
\set tally_a 'a0000000-0000-4000-8000-000000000005'
\set official 'a0000000-0000-4000-8000-0000000000a1'
\set officer 'a0000000-0000-4000-8000-0000000000a2'
\set officer2 'a0000000-0000-4000-8000-0000000000a3'
\set official3 'a0000000-0000-4000-8000-0000000000a4'

INSERT INTO sequent_backend.tenant (id, slug) VALUES
    (:'tenant_a', 'os-test-hsm-a'), (:'tenant_b', 'os-test-hsm-b'), (:'tenant_l', 'os-test-legacy');
INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol) VALUES
    (:'event_a', :'tenant_a', 'RSA256'), (:'event_b', :'tenant_b', 'RSA256'),
    (:'event_l', :'tenant_l', 'RSA256');
INSERT INTO sequent_backend.election (id, tenant_id, election_event_id) VALUES
    (:'election_a', :'tenant_a', :'event_a'), (:'election_l', :'tenant_l', :'event_l');
INSERT INTO sequent_backend.area (id, tenant_id, election_event_id) VALUES
    (:'area_a', :'tenant_a', :'event_a');
INSERT INTO sequent_backend.keys_ceremony (id, tenant_id, election_event_id, trustee_ids, threshold)
VALUES ('a0000000-0000-4000-8000-000000000006', :'tenant_a', :'event_a', '{}', 1);
INSERT INTO sequent_backend.tally_session (id, tenant_id, election_event_id, keys_ceremony_id, threshold)
VALUES (:'tally_a', :'tenant_a', :'event_a', 'a0000000-0000-4000-8000-000000000006', 1);

-- Deployment stays legacy-p12; tenants A and B are promoted individually.
INSERT INTO sequent_backend.operational_signing_required_tenant (tenant_id)
VALUES (:'tenant_a'), (:'tenant_b');

CREATE FUNCTION os_test.request(
    p_id uuid, p_tenant uuid, p_event uuid, p_election uuid, p_scope text, p_payload text,
    p_expires timestamptz
) RETURNS void LANGUAGE sql AS $$
    INSERT INTO sequent_backend.signing_request
        (id, tenant_id, election_event_id, action, election_id, scope_key, subject,
         canonical_payload, payload_sha256, code, rule_revision, rule_snapshot, required,
         requested_by, requested_by_username, expires_at)
    VALUES (p_id, p_tenant, p_event, 'initialize-voting', p_election, p_scope, '{}'::jsonb,
            p_payload, encode(sha256(convert_to(p_payload, 'UTF8')), 'hex'), 'OS-TEST', 1,
            '{}'::jsonb, 1, 'requester', 'requester', p_expires);
$$;
SELECT os_test.request('a0000000-0000-4000-8000-0000000000f1', :'tenant_a', :'event_a',
    :'election_a', 'scope-1', '{"initialize":"a-1"}', clock_timestamp() + interval '1 hour');
SELECT os_test.request('a0000000-0000-4000-8000-0000000000f2', :'tenant_a', :'event_a',
    :'election_a', 'scope-2', '{"initialize":"a-2"}', clock_timestamp() + interval '1 hour');
SELECT os_test.request('b0000000-0000-4000-8000-0000000000f1', :'tenant_b', :'event_b',
    NULL, 'scope-1', '{"initialize":"b-1"}', clock_timestamp() + interval '1 hour');
SELECT os_test.request('c0000000-0000-4000-8000-0000000000f1', :'tenant_l', :'event_l',
    :'election_l', 'scope-1', '{"initialize":"l-1"}', clock_timestamp() + interval '1 hour');

-- Staff issuer and one stored revocation list of event A.
INSERT INTO sequent_backend.certificate_authority
    (id, tenant_id, election_event_id, common_name, subject, issuer_common_name, issuer,
     not_before, not_after, fingerprint_sha256, serial_number, pem, purpose)
VALUES ('a0000000-0000-4000-8000-0000000000c1', :'tenant_a', :'event_a', 'Test staff CA',
        'CN=Test staff CA', 'Test staff CA', 'CN=Test staff CA', now() - interval '1 day',
        now() + interval '1 year', repeat('c1', 32), '01', 'TEST-ANCHOR-PEM', 'staff-signatures');
INSERT INTO sequent_backend.staff_crl
    (tenant_id, election_event_id, issuer_id, issuer_fingerprint, url, der, this_update,
     next_update, status, last_ok_at)
VALUES (:'tenant_a', :'event_a', 'a0000000-0000-4000-8000-0000000000c1', repeat('c1', 32),
        'http://crl.invalid/staff.crl', '\x3000'::bytea, now() - interval '1 hour',
        now() + interval '1 day', 'ok', now() - interval '1 minute');

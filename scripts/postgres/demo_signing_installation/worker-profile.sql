-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-- Exercise the real SECURITY DEFINER chain as the worker, not as the installer.
DO $$
DECLARE
    t uuid := '10000000-0000-4000-8000-000000000001';
    e uuid := '10000000-0000-4000-8000-000000000002';
    r uuid := '10000000-0000-4000-8000-000000000003';
    expected uuid := CASE WHEN current_database()='operational_demo_14990' THEN r END;
BEGIN
    IF current_user <> 'os14990_windmill' OR
       sequent_backend.operational_signing_demo_fixture_id(t,e,'initialize-voting','softhsm-test') IS DISTINCT FROM expected OR
       sequent_backend.operational_signing_demo_fixture_id(t,e,'initialize-voting','yubihsm') IS NOT NULL OR
       sequent_backend.operational_signing_demo_fixture_id(t,e,'transmit-results','softhsm-test') IS NOT NULL OR
       has_table_privilege(current_user,'sequent_backend.operational_signing_demo_install','select') OR
       has_function_privilege(current_user,'sequent_backend.operational_register_demo_scope(uuid,uuid,uuid,text)','execute') THEN
       RAISE EXCEPTION 'worker did not observe only the exact installed capability';
    END IF;
END $$;

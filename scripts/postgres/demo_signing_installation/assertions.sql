-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
INSERT INTO sequent_backend.tenant(id,slug) VALUES ('30000000-0000-4000-8000-000000000001','new');
DO $$
DECLARE demo boolean := current_database()='operational_demo_14990';
    t uuid := '10000000-0000-4000-8000-000000000001'; e uuid := '10000000-0000-4000-8000-000000000002';
    r uuid := '10000000-0000-4000-8000-000000000003'; n uuid := '30000000-0000-4000-8000-000000000001';
BEGIN
    IF (SELECT mode FROM sequent_backend.operational_signing_policy WHERE singleton) <> 'legacy-p12'
       OR sequent_backend.operational_signing_hsm_required(t) IS DISTINCT FROM demo
       OR sequent_backend.operational_signing_hsm_required(n) IS DISTINCT FROM demo
       OR sequent_backend.operational_signing_hsm_required('20000000-0000-4000-8000-000000000001')
       OR sequent_backend.operational_signing_demo_fixture_id(t,e,'initialize-voting','softhsm-test') IS DISTINCT FROM (CASE WHEN demo THEN r END)
       OR sequent_backend.operational_signing_demo_fixture_id(t,e,'initialize-voting','yubihsm') IS NOT NULL
       OR sequent_backend.operational_signing_demo_fixture_id(n,e,'initialize-voting','softhsm-test') IS NOT NULL
       OR sequent_backend.operational_signing_demo_fixture_id(t,n,'initialize-voting','softhsm-test') IS NOT NULL
       OR sequent_backend.operational_signing_demo_fixture_id(t,e,'transmit-results','softhsm-test') IS NOT NULL
       OR has_function_privilege('sequent_os_setup_device','sequent_backend.operational_setup_claim_generation(uuid,uuid,uuid,uuid)','execute') IS DISTINCT FROM (NOT demo)
       OR NOT has_function_privilege('sequent_os_setup_device','sequent_backend.operational_initialize_claim_self_setup(uuid,uuid,uuid,uuid)','execute')
       OR NOT has_function_privilege('sequent_os_initialize_device','sequent_backend.operational_signing_demo_fixture_id(uuid,uuid,text,text)','execute')
       OR has_table_privilege('sequent_os_initialize_device','sequent_backend.operational_signing_demo_install','insert')
       OR has_function_privilege('sequent_os_initialize_device','sequent_backend.operational_register_demo_scope(uuid,uuid,uuid,text)','execute') THEN
       RAISE EXCEPTION 'demo installation crossed its authority, database, tenant, event, action or device';
    END IF;
END $$;

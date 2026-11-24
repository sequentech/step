-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Inert everywhere by default. Only an operator can opt a dedicated,
-- offline demo database into this policy; no JSON tenant setting, import,
-- API, header or environment variable can activate it.
CREATE TABLE sequent_backend.operational_signing_demo_install (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    run_id uuid NOT NULL UNIQUE CHECK (run_id <> '00000000-0000-0000-0000-000000000000'),
    device_profile text NOT NULL CHECK (device_profile IN ('softhsm-test','yubihsm')),
    installed_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    CHECK (current_database() = 'operational_demo_14990')
);
CREATE TABLE sequent_backend.operational_signing_demo_scope (
    tenant_id uuid NOT NULL,
    event_id uuid NOT NULL,
    run_id uuid NOT NULL REFERENCES sequent_backend.operational_signing_demo_install(run_id),
    PRIMARY KEY (tenant_id,event_id),
    FOREIGN KEY (tenant_id,event_id) REFERENCES sequent_backend.election_event(tenant_id,id)
);
REVOKE ALL ON TABLE sequent_backend.operational_signing_demo_install,
    sequent_backend.operational_signing_demo_scope FROM PUBLIC;

CREATE FUNCTION sequent_backend.operational_demo_immutable()
RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog AS $$
BEGIN
    RAISE EXCEPTION 'operational demo installation is immutable; create a fresh isolated installation';
END $$;
REVOKE ALL ON FUNCTION sequent_backend.operational_demo_immutable() FROM PUBLIC;
CREATE TRIGGER operational_demo_install_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON sequent_backend.operational_signing_demo_install FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.operational_demo_immutable();
CREATE TRIGGER operational_demo_scope_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON sequent_backend.operational_signing_demo_scope FOR EACH STATEMENT
    EXECUTE FUNCTION sequent_backend.operational_demo_immutable();

-- This trigger is never executed on a hosted database. In the dedicated
-- database, it remains inactive until the installation row exists. Existing
-- unrelated tenants stay unchanged. A fresh tenant is HSM-required before
-- its creating transaction can finish, but cannot sign in an unadmitted event.
CREATE FUNCTION sequent_backend.operational_demo_new_tenant()
RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog AS $$
DECLARE v_profile text;
BEGIN
    IF current_database() <> 'operational_demo_14990' THEN RETURN NEW; END IF;
    SELECT device_profile INTO v_profile FROM sequent_backend.operational_signing_demo_install WHERE singleton;
    IF v_profile IS NOT NULL THEN
        INSERT INTO sequent_backend.operational_signing_required_tenant(tenant_id) VALUES (NEW.id);
        INSERT INTO sequent_backend.operational_setup_installation(tenant_id,device_profile) VALUES (NEW.id,v_profile);
    END IF;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION sequent_backend.operational_demo_new_tenant() FROM PUBLIC;
CREATE TRIGGER operational_demo_new_tenant AFTER INSERT ON sequent_backend.tenant
    FOR EACH ROW WHEN (current_database() = 'operational_demo_14990')
    EXECUTE FUNCTION sequent_backend.operational_demo_new_tenant();

-- Operator-only transaction. No SECURITY DEFINER: the caller needs the actual
-- deployment DDL owner's rights. Enrolling oneself/creating a tenant is not
-- permission to mark a database or register an event.
CREATE FUNCTION sequent_backend.operational_register_demo_scope(
    p_tenant uuid,p_event uuid,p_run uuid,p_profile text
) RETURNS void LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog AS $$
BEGIN
    IF current_database() IS DISTINCT FROM 'operational_demo_14990' THEN
        RAISE EXCEPTION 'dedicated demo database only';
    END IF;
    IF p_run IS NULL OR p_run='00000000-0000-0000-0000-000000000000'
       OR p_profile IS NULL OR p_profile NOT IN ('softhsm-test','yubihsm')
       OR p_tenant IS NULL OR p_event IS NULL THEN RAISE EXCEPTION 'invalid demo installation'; END IF;
    -- Block concurrent tenant writers across the first installation so none
    -- can miss the default. Legacy approve holds the matching policy lock.
    LOCK TABLE sequent_backend.tenant IN SHARE ROW EXCLUSIVE MODE;
    PERFORM pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment',0));
    IF NOT EXISTS (SELECT FROM sequent_backend.election_event WHERE (tenant_id,id)=(p_tenant,p_event)) THEN
        RAISE EXCEPTION 'unknown demo event';
    END IF;
    IF EXISTS (SELECT FROM sequent_backend.operational_signing_demo_install
        WHERE (run_id,device_profile) IS DISTINCT FROM (p_run,p_profile)) THEN
        RAISE EXCEPTION 'installation identity cannot change';
    END IF;
    IF EXISTS (SELECT FROM sequent_backend.operational_setup_installation WHERE tenant_id=p_tenant AND device_profile<>p_profile) THEN
        RAISE EXCEPTION 'conflicting tenant device installation';
    END IF;
    IF to_regprocedure('sequent_backend.operational_initialize_claim_self_setup(uuid,uuid,uuid,uuid)') IS NULL THEN
        RAISE EXCEPTION 'install the request-bound enrollment migration before enabling the demo';
    END IF;
    -- Never alter privileges for hosted installations. In this explicitly
    -- dedicated database the device must use the request-bound claim API,
    -- not the existing general Security Officer workflow. Its definer still
    -- owns the original function. This operation also reasserts on rerun.
    REVOKE EXECUTE ON FUNCTION sequent_backend.operational_setup_claim_generation(uuid,uuid,uuid,uuid)
        FROM sequent_os_setup_device;
    IF EXISTS (SELECT FROM sequent_backend.operational_signing_demo_scope
               WHERE (tenant_id,event_id,run_id)=(p_tenant,p_event,p_run)) THEN RETURN; END IF;
    -- Don't strand legacy work or convert software approvals from an earlier
    -- context. An operator must first finish/cancel the pending requests using
    -- the ordinary audited workflow. Never cancel anything from this function.
    IF EXISTS (SELECT FROM sequent_backend.signing_request r WHERE r.tenant_id=p_tenant
           AND (r.status='waiting' OR (r.status='completed' AND r.executed_at IS NULL))
           AND (r.election_event_id=p_event OR NOT EXISTS (
                SELECT FROM sequent_backend.operational_signing_demo_scope WHERE tenant_id=p_tenant))) THEN
        RAISE EXCEPTION 'finish or cancel existing signing work before enabling the demo policy';
    END IF;
    INSERT INTO sequent_backend.operational_signing_demo_install(run_id,device_profile)
        VALUES (p_run,p_profile) ON CONFLICT (singleton) DO NOTHING;
    INSERT INTO sequent_backend.operational_signing_required_tenant(tenant_id)
        VALUES (p_tenant) ON CONFLICT DO NOTHING;
    INSERT INTO sequent_backend.operational_setup_installation(tenant_id,device_profile)
        VALUES (p_tenant,p_profile) ON CONFLICT DO NOTHING;
    INSERT INTO sequent_backend.operational_signing_demo_scope(tenant_id,event_id,run_id)
        VALUES (p_tenant,p_event,p_run);
END $$;
REVOKE ALL ON FUNCTION sequent_backend.operational_register_demo_scope(uuid,uuid,uuid,text) FROM PUBLIC;

-- Only return the installed run for the exact exposed operation. This grants
-- neither signing nor table access. Service binaries compare the result to
-- an independently provisioned launch manifest/run-id and pinned device.
CREATE FUNCTION sequent_backend.operational_signing_demo_fixture_id(
    p_tenant uuid,p_event uuid,p_action text,p_profile text
) RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog AS $$
    SELECT s.run_id FROM sequent_backend.operational_signing_demo_scope s
      JOIN sequent_backend.operational_signing_demo_install i ON i.run_id=s.run_id
      JOIN sequent_backend.operational_setup_installation p ON p.tenant_id=s.tenant_id
      WHERE current_database()='operational_demo_14990' AND i.singleton
        AND (s.tenant_id,s.event_id)=(p_tenant,p_event) AND p_action='initialize-voting'
        AND p.device_profile=p_profile AND i.device_profile=p_profile
        AND sequent_backend.operational_signing_hsm_required(p_tenant);
$$;
GRANT CREATE ON SCHEMA sequent_backend TO sequent_os_ledger_api_owner;
GRANT SELECT ON sequent_backend.operational_signing_demo_install,
    sequent_backend.operational_signing_demo_scope TO sequent_os_ledger_api_owner;
ALTER FUNCTION sequent_backend.operational_signing_demo_fixture_id(uuid,uuid,text,text)
    OWNER TO sequent_os_ledger_api_owner;
REVOKE CREATE ON SCHEMA sequent_backend FROM sequent_os_ledger_api_owner;
REVOKE ALL ON FUNCTION sequent_backend.operational_signing_demo_fixture_id(uuid,uuid,text,text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION sequent_backend.operational_signing_demo_fixture_id(uuid,uuid,text,text)
    TO sequent_os_approve,sequent_os_initialize_orchestrator,sequent_os_initialize_device,
       sequent_os_initialize_ca,sequent_os_setup_device,sequent_os_setup_authority;

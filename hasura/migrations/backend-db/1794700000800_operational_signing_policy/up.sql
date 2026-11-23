-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Deployment admin owns promotion. Not tracked by Hasura. Service roles only
-- need policy reads; grant them explicitly if separate from the migration owner.
CREATE TABLE sequent_backend.operational_signing_policy (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    mode text NOT NULL CHECK (mode IN ('legacy-p12', 'hsm-required'))
);
INSERT INTO sequent_backend.operational_signing_policy (mode) VALUES ('legacy-p12');

CREATE TABLE sequent_backend.operational_signing_required_tenant (
    tenant_id uuid PRIMARY KEY
);

CREATE FUNCTION sequent_backend.protect_operational_signing_policy()
RETURNS trigger LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog
AS $$
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('operational-signing-policy:deployment', 0));
    IF TG_TABLE_NAME = 'operational_signing_policy' THEN
        IF TG_OP = 'UPDATE' AND OLD.mode = 'legacy-p12' AND NEW.mode = 'hsm-required'
           AND OLD.singleton = NEW.singleton THEN
            RETURN NEW;
        END IF;
    ELSIF TG_OP = 'INSERT' THEN
        PERFORM FROM sequent_backend.operational_signing_policy WHERE singleton;
        IF NOT FOUND THEN RAISE EXCEPTION 'missing operational signing policy'; END IF;
        IF NOT EXISTS (SELECT FROM sequent_backend.tenant WHERE id = NEW.tenant_id) THEN
            RAISE EXCEPTION 'unknown tenant for operational signing policy';
        END IF;
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'operational signing policy cannot be downgraded or deleted';
END;
$$;
CREATE TRIGGER operational_signing_policy_monotonic
BEFORE INSERT OR UPDATE OR DELETE ON sequent_backend.operational_signing_policy
FOR EACH ROW EXECUTE FUNCTION sequent_backend.protect_operational_signing_policy();
CREATE TRIGGER operational_signing_policy_no_truncate
BEFORE TRUNCATE ON sequent_backend.operational_signing_policy
FOR EACH STATEMENT EXECUTE FUNCTION sequent_backend.protect_operational_signing_policy();
CREATE TRIGGER operational_signing_tenant_monotonic
BEFORE INSERT OR UPDATE OR DELETE ON sequent_backend.operational_signing_required_tenant
FOR EACH ROW EXECUTE FUNCTION sequent_backend.protect_operational_signing_policy();
CREATE TRIGGER operational_signing_tenant_no_truncate
BEFORE TRUNCATE ON sequent_backend.operational_signing_required_tenant
FOR EACH STATEMENT EXECUTE FUNCTION sequent_backend.protect_operational_signing_policy();
REVOKE ALL ON TABLE sequent_backend.operational_signing_policy,
    sequent_backend.operational_signing_required_tenant FROM PUBLIC;
REVOKE ALL ON FUNCTION sequent_backend.protect_operational_signing_policy() FROM PUBLIC;

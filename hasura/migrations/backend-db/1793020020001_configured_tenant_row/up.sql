-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- A tenant row that carries configuration_signing keeps its id and is never
-- deleted, so no insert can bring the tenant back with other trust or none.
CREATE FUNCTION sequent_backend.guard_configured_tenant_row()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF OLD.settings -> 'configuration_signing' IS NOT NULL
       AND (TG_OP = 'DELETE' OR NEW.id IS DISTINCT FROM OLD.id) THEN
        RAISE EXCEPTION 'configuration_signing_fixed' USING ERRCODE = '42501',
            DETAIL = 'A tenant with package-signing configuration cannot be deleted or change its id.';
    END IF;
    IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER guard_configured_tenant_row
BEFORE DELETE OR UPDATE OF id ON sequent_backend.tenant
FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_configured_tenant_row();

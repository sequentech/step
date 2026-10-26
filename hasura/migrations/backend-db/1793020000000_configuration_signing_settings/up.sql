-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Applies to every write path, including direct Hasura mutations and restores.
-- X.509 chain/signature validation remains at the package-import boundary.
CREATE FUNCTION sequent_backend.guard_configuration_signing_settings()
RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    previous jsonb;
    proposed jsonb;
    roots text;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        previous := OLD.settings -> 'configuration_signing';
    END IF;
    proposed := NEW.settings -> 'configuration_signing';
    IF proposed IS NOT DISTINCT FROM previous THEN RETURN NEW; END IF;

    PERFORM pg_advisory_xact_lock(hashtextextended('configuration-package-import:' || NEW.id::text, 0));
    IF proposed IS NULL OR jsonb_typeof(proposed) <> 'object'
       OR proposed ->> 'policy' IS DISTINCT FROM 'required'
       OR jsonb_typeof(proposed -> 'required_approvals') IS DISTINCT FROM 'number'
       OR proposed ->> 'required_approvals' !~ '^[0-9]+$'
       OR (proposed ->> 'required_approvals')::numeric NOT BETWEEN 2 AND 65535 THEN
        RAISE EXCEPTION 'configuration_signing_invalid' USING ERRCODE = '23514',
            DETAIL = 'Signed packages and 2 to 65535 distinct approvals are required.';
    END IF;
    FOREACH roots IN ARRAY ARRAY['package_roots', 'staff_roots'] LOOP
        IF jsonb_typeof(proposed -> roots) IS DISTINCT FROM 'string'
           OR length(proposed ->> roots) > 262144
           OR (proposed ->> roots) !~ '^\s*(-----BEGIN CERTIFICATE-----[\r\n]+[A-Za-z0-9+/=\r\n]+-----END CERTIFICATE-----\s*)+$' THEN
            RAISE EXCEPTION 'configuration_signing_invalid' USING ERRCODE = '23514',
                DETAIL = 'Both public CA certificate bundles must be nonempty PEM files, at most 256 KiB.';
        END IF;
    END LOOP;
    IF previous IS NOT NULL AND (
        proposed -> 'policy' IS DISTINCT FROM previous -> 'policy'
        OR proposed -> 'package_roots' IS DISTINCT FROM previous -> 'package_roots'
        OR proposed -> 'required_approvals' IS DISTINCT FROM previous -> 'required_approvals'
    ) THEN
        RAISE EXCEPTION 'configuration_signing_fixed' USING ERRCODE = '42501',
            DETAIL = 'Package-signing CAs and approval count cannot change after initial configuration.';
    END IF;
    IF EXISTS (SELECT 1 FROM sequent_backend.election_event WHERE tenant_id = NEW.id AND is_archived IS DISTINCT FROM true) THEN
        RAISE EXCEPTION 'configuration_signing_active_elections' USING ERRCODE = '42501',
            DETAIL = 'Configure trust before the first election; replace staff CAs only after every tenant election is archived.';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER guard_configuration_signing_settings
BEFORE INSERT OR UPDATE OF settings ON sequent_backend.tenant
FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_configuration_signing_settings();

-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- This flag is initialization evidence, not editable configuration. Hasura
-- whole-record saves may preserve it, but only report completion can change it.
CREATE OR REPLACE FUNCTION sequent_backend.guard_initialization_report_flag()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    previous boolean := false;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        previous := COALESCE(OLD.initialization_report_generated, false);
    END IF;
    IF previous IS NOT DISTINCT FROM COALESCE(NEW.initialization_report_generated, false)
       OR sequent_backend.trusted_write_allowed() THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'Initialization report state can only change through report generation'
        USING ERRCODE = '42501',
              HINT = 'Generate the initialization report from the Publish tab. Reload a stale form before saving.';
END;
$$;

CREATE TRIGGER guard_initialization_report_flag
BEFORE INSERT OR UPDATE OF initialization_report_generated ON sequent_backend.election
FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_initialization_report_flag();

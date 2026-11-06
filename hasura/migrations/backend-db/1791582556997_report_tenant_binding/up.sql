-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- A report stays in the tenant it was created in, and its election event and
-- election must belong to that tenant. Rows saved earlier are checked only when
-- these columns change, so schedule bookkeeping on them keeps working.
CREATE OR REPLACE FUNCTION "sequent_backend"."check_report_tenant_binding"()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        IF NEW.tenant_id IS DISTINCT FROM OLD.tenant_id THEN
            RAISE EXCEPTION 'A report cannot move to another tenant'
                USING ERRCODE = 'check_violation';
        END IF;
        IF NEW.election_event_id IS NOT DISTINCT FROM OLD.election_event_id
            AND NEW.election_id IS NOT DISTINCT FROM OLD.election_id THEN
            RETURN NEW;
        END IF;
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM "sequent_backend"."election_event"
        WHERE id = NEW.election_event_id
            AND tenant_id = NEW.tenant_id
    ) THEN
        RAISE EXCEPTION 'Report election event % does not belong to tenant %',
            NEW.election_event_id, NEW.tenant_id
            USING ERRCODE = 'foreign_key_violation';
    END IF;

    IF NEW.election_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM "sequent_backend"."election"
        WHERE id = NEW.election_id
            AND election_event_id = NEW.election_event_id
            AND tenant_id = NEW.tenant_id
    ) THEN
        RAISE EXCEPTION 'Report election % does not belong to election event %',
            NEW.election_id, NEW.election_event_id
            USING ERRCODE = 'foreign_key_violation';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS "check_report_tenant_binding"
ON "sequent_backend"."report";

CREATE TRIGGER "check_report_tenant_binding"
BEFORE INSERT OR UPDATE OF tenant_id, election_event_id, election_id
ON "sequent_backend"."report"
FOR EACH ROW
EXECUTE FUNCTION "sequent_backend"."check_report_tenant_binding"();

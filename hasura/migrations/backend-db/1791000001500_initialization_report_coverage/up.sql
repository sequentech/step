-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Private, untracked evidence of the actual generated ballot styles an
-- initialization report executes, captured in the session creation transaction.
-- Per-Post empty lists are explicit evidence; absence predates this proof.
CREATE TABLE sequent_backend.initialization_report_coverage (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    tally_session_id uuid NOT NULL,
    coverage jsonb NOT NULL CHECK (jsonb_typeof(coverage) = 'object'),
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (tenant_id, election_event_id, tally_session_id),
    FOREIGN KEY (tenant_id, election_event_id)
        REFERENCES sequent_backend.election_event (tenant_id, id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id, election_event_id, tally_session_id)
        REFERENCES sequent_backend.tally_session (tenant_id, election_event_id, id) ON DELETE CASCADE
);

CREATE FUNCTION sequent_backend.guard_initialization_report_coverage()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        IF NEW IS NOT DISTINCT FROM OLD THEN RETURN NEW; END IF;
        RAISE EXCEPTION 'Initialization report coverage is immutable' USING ERRCODE = '42501';
    END IF;
    IF NOT sequent_backend.trusted_write_allowed() THEN
        RAISE EXCEPTION 'Initialization report coverage requires an authorized server write'
            USING ERRCODE = '42501';
    END IF;
    IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER guard_initialization_report_coverage
BEFORE INSERT OR UPDATE OR DELETE ON sequent_backend.initialization_report_coverage
FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_initialization_report_coverage();

-- Selection and decryption inputs stay fixed once private report coverage exists.
-- Progress/status and unrelated annotations remain editable; trusted event cleanup
-- and validated server workflows retain their transaction-scoped exception.
CREATE FUNCTION sequent_backend.guard_initialization_report_session()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF sequent_backend.trusted_write_allowed() THEN
        IF TG_OP = 'DELETE' THEN RETURN OLD; END IF;
        RETURN NEW;
    END IF;
    IF TG_OP = 'DELETE' THEN
        IF EXISTS (
            SELECT 1 FROM sequent_backend.initialization_report_coverage c
            WHERE c.tenant_id = OLD.tenant_id
              AND c.election_event_id = OLD.election_event_id
              AND c.tally_session_id = OLD.id
        ) THEN
            RAISE EXCEPTION 'Initialization report selection can only change through the server workflow'
                USING ERRCODE = '42501', HINT = 'Generate a new initialization report to change its selection.';
        END IF;
        RETURN OLD;
    END IF;
    IF ROW(OLD.id, OLD.tenant_id, OLD.election_event_id, OLD.election_ids,
           OLD.area_ids, OLD.tally_type, OLD.keys_ceremony_id, OLD.configuration,
           OLD.threshold)
       IS DISTINCT FROM
       ROW(NEW.id, NEW.tenant_id, NEW.election_event_id, NEW.election_ids,
           NEW.area_ids, NEW.tally_type, NEW.keys_ceremony_id, NEW.configuration,
           NEW.threshold)
       AND EXISTS (
           SELECT 1 FROM sequent_backend.initialization_report_coverage c
           WHERE (c.tenant_id = OLD.tenant_id
                  AND c.election_event_id = OLD.election_event_id
                  AND c.tally_session_id = OLD.id)
              OR (c.tenant_id = NEW.tenant_id
                  AND c.election_event_id = NEW.election_event_id
                  AND c.tally_session_id = NEW.id)
       ) THEN
        RAISE EXCEPTION 'Initialization report selection can only change through the server workflow'
            USING ERRCODE = '42501', HINT = 'Generate a new initialization report to change its selection.';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER guard_initialization_report_session
BEFORE UPDATE OR DELETE ON sequent_backend.tally_session
FOR EACH ROW EXECUTE FUNCTION sequent_backend.guard_initialization_report_session();

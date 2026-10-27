-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Safety state belongs to the TSF, separately from the editable event settings.
CREATE TABLE sequent_backend.election_lifecycle (
    tenant_id uuid NOT NULL REFERENCES sequent_backend.tenant(id),
    election_event_id uuid PRIMARY KEY REFERENCES sequent_backend.election_event(id) ON DELETE CASCADE,
    phase text NOT NULL DEFAULT 'preparation' CHECK (phase IN ('preparation','execution','evaluation','post_processing')),
    safety text NOT NULL DEFAULT 'recoverable' CHECK (safety IN ('operational','recoverable','unrecoverable')),
    generation bigint NOT NULL DEFAULT 0 CHECK (generation >= 0),
    reason text,
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    last_monitored_at timestamptz NOT NULL DEFAULT to_timestamp(0),
    UNIQUE(tenant_id,election_event_id)
);
CREATE TABLE sequent_backend.lifecycle_service_lease (
    component text PRIMARY KEY CHECK (component IN ('harvest','windmill')),
    expires_at timestamptz NOT NULL,
    started_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE sequent_backend.lifecycle_notice (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL REFERENCES sequent_backend.election_event(id) ON DELETE CASCADE,
    kind text NOT NULL,
    details jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    logged_at timestamptz,
    UNIQUE(tenant_id,election_event_id,id)
);
INSERT INTO sequent_backend.election_lifecycle(tenant_id,election_event_id,phase,reason)
SELECT tenant_id,id,
    CASE WHEN status->>'voting_status' IN ('OPEN','PAUSED') THEN 'execution'
         WHEN status->>'voting_status' = 'CLOSED' THEN 'evaluation'
         ELSE 'preparation' END,
    'Startup self-tests and election-board recovery are required'
FROM sequent_backend.election_event;

CREATE FUNCTION sequent_backend.seed_election_lifecycle() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    INSERT INTO sequent_backend.election_lifecycle(tenant_id,election_event_id,reason)
    VALUES(NEW.tenant_id,NEW.id,'Startup self-tests are required');
    RETURN NEW;
END;
$$;
CREATE TRIGGER seed_election_lifecycle AFTER INSERT ON sequent_backend.election_event
FOR EACH ROW EXECUTE FUNCTION sequent_backend.seed_election_lifecycle();

CREATE FUNCTION sequent_backend.guard_election_lifecycle() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    -- A new event receives only the fail-closed initial state, never a caller's state.
    IF TG_OP='INSERT' AND pg_trigger_depth()>1 AND NEW.phase='preparation'
       AND NEW.safety='recoverable' AND NEW.generation=0 THEN RETURN NEW; END IF;
    IF NOT sequent_backend.trusted_write_allowed() THEN
        RAISE EXCEPTION 'Election lifecycle changes require the server workflow' USING ERRCODE='42501';
    END IF;
    IF TG_OP='DELETE' THEN RETURN OLD; END IF;
    IF NOT EXISTS (SELECT 1 FROM sequent_backend.election_event e
                   WHERE e.id=NEW.election_event_id AND e.tenant_id=NEW.tenant_id) THEN
        RAISE EXCEPTION 'Election lifecycle tenant mismatch' USING ERRCODE='23514';
    END IF;
    IF TG_OP='UPDATE' THEN
        IF OLD.tenant_id<>NEW.tenant_id OR OLD.election_event_id<>NEW.election_event_id
           OR NEW.generation<OLD.generation THEN
            RAISE EXCEPTION 'Election lifecycle identity and generation cannot be reset' USING ERRCODE='42501';
        END IF;
        IF OLD.safety='unrecoverable' AND NEW.safety<>'unrecoverable' THEN
            RAISE EXCEPTION 'An unrecoverable election cannot be resumed' USING ERRCODE='42501';
        END IF;
        IF (OLD.phase='preparation' AND NEW.phase='post_processing')
           OR (OLD.phase='execution' AND NEW.phase IN ('preparation','post_processing'))
           OR (OLD.phase='evaluation' AND NEW.phase NOT IN ('evaluation','post_processing'))
           OR (OLD.phase='post_processing' AND NEW.phase<>'post_processing') THEN
            RAISE EXCEPTION 'Electoral phase cannot move backwards' USING ERRCODE='42501';
        END IF;
    END IF;
    NEW.updated_at=clock_timestamp();
    RETURN NEW;
END;
$$;
CREATE TRIGGER guard_election_lifecycle BEFORE INSERT OR UPDATE OR DELETE
ON sequent_backend.election_lifecycle FOR EACH ROW
EXECUTE FUNCTION sequent_backend.guard_election_lifecycle();

-- Remains fail closed until the audit-storage mirror migration is installed.
CREATE FUNCTION sequent_backend.audit_storage_allows_voters() RETURNS boolean
LANGUAGE sql VOLATILE AS $$ SELECT false $$;

-- Reads by the GraphQL voter role use this view as well as the action guards.
-- A dead worker, missing test, stale test or missing lease is a refusal.
CREATE VIEW sequent_backend.election_lifecycle_access AS
SELECT l.tenant_id,l.election_event_id,l.phase,l.safety,l.generation,
       false AS voter_allowed
FROM sequent_backend.election_lifecycle l;
-- The self-test migration replaces this provisional fail-closed view once its
-- tables exist. It is safe even between the two migrations.

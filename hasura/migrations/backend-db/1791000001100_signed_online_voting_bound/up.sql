-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Keep ballot acceptance independent of beat and of mutable live schedule
-- rows. This private projection contains the newest executed configuration's
-- effective START/END rows at each Post; it is not tracked in Hasura.
-- Parse configuration when approvals/Posts change, never for every ballot.
LOCK TABLE sequent_backend.signing_request IN SHARE ROW EXCLUSIVE MODE;
LOCK TABLE sequent_backend.election IN SHARE ROW EXCLUSIVE MODE;

-- Keep per-ballot authorization reads narrow: never fetch/decompress the
-- schedule in a retained snapshot or executed subject just to read its policy.
ALTER TABLE sequent_backend.lifecycle_snapshot
    ADD COLUMN snapshot_close_required boolean GENERATED ALWAYS AS
        (COALESCE((snapshot #>> '{close_voting,required}')::boolean, false)) STORED,
    ADD COLUMN snapshot_unsigned_close_policy text GENERATED ALWAYS AS
        (COALESCE(snapshot #>> '{policies,unsigned_scheduled_close}', 'refuse')) STORED;
ALTER TABLE sequent_backend.signing_request
    ADD COLUMN subject_close_required boolean GENERATED ALWAYS AS
        (CASE WHEN action = 'approve-configuration'
         THEN COALESCE((subject #>> '{close_voting,required}')::boolean, false) END) STORED,
    ADD COLUMN subject_unsigned_close_policy text GENERATED ALWAYS AS
        (CASE WHEN action = 'approve-configuration'
         THEN COALESCE(subject #>> '{policies,unsigned_scheduled_close}', 'refuse') END) STORED,
    ADD COLUMN subject_publication_id text GENERATED ALWAYS AS
        (CASE WHEN action = 'approve-configuration'
         THEN subject->>'ballot_publication_id' END) STORED;

CREATE INDEX lifecycle_snapshot_close_authority_scope
    ON sequent_backend.lifecycle_snapshot
        (tenant_id, election_event_id, election_id, created_at DESC, id DESC);

-- An uncovered live close may bound ballots only when the same two copies
-- allow its execution. Signed snapshots read the completed/executed subject, not an
-- editable presentation or a forged replacement of the kept snapshot fields.
CREATE FUNCTION sequent_backend.live_voting_close_allowed(
    target_tenant uuid, target_event uuid, target_post uuid
) RETURNS boolean LANGUAGE sql STABLE AS $$
    SELECT CASE WHEN kept.approval_request_id IS NOT NULL AND approved.id IS NULL THEN false
        ELSE NOT (COALESCE(current_rule.requirement = 'required', false)
                OR CASE WHEN kept.approval_request_id IS NULL
                        THEN COALESCE(kept.snapshot_close_required, false)
                        ELSE approved.subject_close_required END)
        OR (COALESCE(event.presentation #>> '{lifecycle_policies,unsigned_scheduled_close}', 'refuse') = 'run-as-system'
            AND CASE WHEN kept.approval_request_id IS NULL
                     THEN COALESCE(kept.snapshot_unsigned_close_policy, 'refuse')
                     ELSE approved.subject_unsigned_close_policy END = 'run-as-system') END
    FROM (VALUES (1)) seed(dummy)
    LEFT JOIN sequent_backend.election_event event
        ON event.tenant_id = target_tenant AND event.id = target_event
    LEFT JOIN sequent_backend.signing_rule current_rule
        ON current_rule.tenant_id = target_tenant AND current_rule.election_event_id = target_event
       AND current_rule.action = 'close-voting'
    LEFT JOIN LATERAL (
        SELECT tenant_id, election_event_id, election_id, approval_request_id, ballot_publication_id,
               snapshot_close_required, snapshot_unsigned_close_policy
        FROM (
            (SELECT tenant_id, election_event_id, election_id, approval_request_id, ballot_publication_id,
                    snapshot_close_required, snapshot_unsigned_close_policy, created_at, id
             FROM sequent_backend.lifecycle_snapshot
             WHERE tenant_id = target_tenant AND election_event_id = target_event
               AND election_id IS NULL
             ORDER BY created_at DESC, id DESC LIMIT 1)
            UNION ALL
            (SELECT tenant_id, election_event_id, election_id, approval_request_id, ballot_publication_id,
                    snapshot_close_required, snapshot_unsigned_close_policy, created_at, id
             FROM sequent_backend.lifecycle_snapshot
             WHERE tenant_id = target_tenant AND election_event_id = target_event
               AND election_id = target_post
             ORDER BY created_at DESC, id DESC LIMIT 1)
        ) applicable
        ORDER BY created_at DESC, id DESC LIMIT 1
    ) kept ON true
    LEFT JOIN sequent_backend.signing_request approved
        ON approved.id = kept.approval_request_id
       AND approved.tenant_id = target_tenant AND approved.election_event_id = target_event
       AND approved.action = 'approve-configuration'
       AND approved.status IN ('completed', 'executed') AND approved.completed_at IS NOT NULL
       AND approved.scope_key = '|||' || COALESCE(kept.election_id::text, 'event')
       AND CASE
           WHEN approved.subject_publication_id ~* '^([0-9a-f]{32}|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}|\{[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\}|urn:uuid:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$'
           THEN replace(lower(approved.subject_publication_id), 'urn:uuid:', '')::uuid
           ELSE NULL END = kept.ballot_publication_id
$$;

CREATE TABLE sequent_backend.signed_voting_boundary (
    tenant_id uuid NOT NULL,
    election_event_id uuid NOT NULL,
    election_id uuid NOT NULL,
    approval_request_id uuid NOT NULL,
    scheduled_event_id text NOT NULL,
    event_processor text NOT NULL CHECK (event_processor IN ('START_VOTING_PERIOD', 'END_VOTING_PERIOD')),
    channels jsonb NOT NULL CHECK (jsonb_typeof(channels) = 'array'),
    source_timezone text,
    scheduled_date text NOT NULL,
    scheduled_at timestamptz NOT NULL,
    PRIMARY KEY (tenant_id, election_event_id, election_id, scheduled_event_id)
);
CREATE INDEX signed_voting_boundary_bound
    ON sequent_backend.signed_voting_boundary
        (tenant_id, election_event_id, election_id, event_processor, scheduled_at);
CREATE INDEX signing_request_executed_configuration_scope
    ON sequent_backend.signing_request
        (tenant_id, election_event_id, scope_key, executed_at DESC, id)
    WHERE action = 'approve-configuration' AND status = 'executed';

-- Match signed_instant: a missing/invalid signed date does not execute a row.
CREATE FUNCTION sequent_backend.signed_voting_instant(value text)
RETURNS timestamptz LANGUAGE plpgsql IMMUTABLE STRICT AS $$
BEGIN
    IF value !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}[Tt]([01][0-9]|2[0-3]):[0-5][0-9]:([0-5][0-9]|60)([.][0-9]+)?([Zz]|[+-][0-9]{2}:[0-9]{2})$' THEN
        RETURN NULL;
    END IF;
    RETURN value::timestamptz;
EXCEPTION WHEN data_exception THEN
    RETURN NULL;
END;
$$;

-- Match ScheduledRow.payload().channels(), including the legacy default.
CREATE FUNCTION sequent_backend.signed_voting_channels(transition jsonb)
RETURNS jsonb LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    selected jsonb := transition->'voting_channels';
BEGIN
    IF selected IS NULL OR selected IN ('null'::jsonb, '[]'::jsonb) THEN
        RETURN '["ONLINE", "KIOSK"]'::jsonb;
    END IF;
    IF jsonb_typeof(selected) <> 'array'
       OR NOT selected <@ '["ONLINE", "KIOSK", "EARLY_VOTING", "TELEPHONE"]'::jsonb THEN
        RAISE EXCEPTION 'Invalid voting channels in signed configuration'
            USING ERRCODE = '23514';
    END IF;
    RETURN (SELECT jsonb_agg(channel)
        FROM jsonb_array_elements_text('["ONLINE", "KIOSK", "EARLY_VOTING", "TELEPHONE"]'::jsonb) channel
        WHERE selected ? channel);
END;
$$;

CREATE FUNCTION sequent_backend.refresh_signed_voting_boundary(
    target_tenant uuid, target_event uuid
) RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    -- Signing effects already own this lock. An election INSERT/reparenting
    -- trigger may own a child row first: refuse contention rather than wait
    -- in the inverse order or commit a projection from an older approval.
    IF NOT pg_try_advisory_xact_lock(hashtextextended(
        format('signing-event:%s:%s', target_tenant, target_event), 0
    )) THEN
        RAISE EXCEPTION 'Signed voting configuration is busy; retry the election change'
            USING ERRCODE = '55P03';
    END IF;
    DELETE FROM sequent_backend.signed_voting_boundary
    WHERE tenant_id = target_tenant AND election_event_id = target_event;

    INSERT INTO sequent_backend.signed_voting_boundary
        (tenant_id, election_event_id, election_id, approval_request_id,
         scheduled_event_id, event_processor, channels, source_timezone, scheduled_date, scheduled_at)
    WITH latest AS (
        SELECT election.id AS post_id, approval.id AS approval_id, approval.subject
        FROM sequent_backend.election election
        CROSS JOIN LATERAL (
            SELECT id, subject
            FROM sequent_backend.signing_request
            WHERE tenant_id = target_tenant AND election_event_id = target_event
              AND action = 'approve-configuration' AND status = 'executed'
              AND scope_key IN ('|||event', '|||' || election.id::text)
            ORDER BY executed_at DESC, id
            LIMIT 1
        ) approval
        WHERE election.tenant_id = target_tenant AND election.election_event_id = target_event
    ), schedule AS (
        SELECT latest.post_id, latest.approval_id, transition,
               sequent_backend.signed_voting_channels(transition) AS channels
        FROM latest
        CROSS JOIN LATERAL jsonb_array_elements(
            CASE WHEN jsonb_typeof(subject->'schedule') = 'array'
                 THEN subject->'schedule' ELSE '[]'::jsonb END
        ) transition
        WHERE transition->>'event_processor' IN ('START_VOTING_PERIOD', 'END_VOTING_PERIOD')
    ), effective AS (
        SELECT schedule.*, sequent_backend.signed_voting_instant(transition->>'scheduled_date') AS instant
        FROM schedule
        WHERE transition->>'election_id' = post_id::text
           OR (transition->>'election_id' IS NULL AND NOT EXISTS (
                SELECT 1 FROM schedule own
                WHERE own.post_id = schedule.post_id
                  AND own.transition->>'event_processor' = schedule.transition->>'event_processor'
                  AND own.transition->>'election_id' = schedule.post_id::text
                  AND own.channels ? 'ONLINE'
           ))
    )
    SELECT target_tenant, target_event, post_id, approval_id,
           transition->>'scheduled_event_id', transition->>'event_processor',
           channels, transition->>'timezone', transition->>'scheduled_date', instant
    FROM effective WHERE instant IS NOT NULL;
END;
$$;

CREATE FUNCTION sequent_backend.update_signed_voting_approval()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP <> 'INSERT' AND OLD.action = 'approve-configuration' AND OLD.status = 'executed' THEN
        PERFORM sequent_backend.refresh_signed_voting_boundary(OLD.tenant_id, OLD.election_event_id);
    END IF;
    IF TG_OP <> 'DELETE' AND NEW.action = 'approve-configuration' AND NEW.status = 'executed'
       AND (TG_OP = 'INSERT' OR OLD.action <> 'approve-configuration' OR OLD.status <> 'executed'
            OR ROW(OLD.tenant_id, OLD.election_event_id) IS DISTINCT FROM ROW(NEW.tenant_id, NEW.election_event_id)) THEN
        PERFORM sequent_backend.refresh_signed_voting_boundary(NEW.tenant_id, NEW.election_event_id);
    END IF;
    RETURN NULL;
END;
$$;
CREATE TRIGGER update_signed_voting_approval
AFTER INSERT OR UPDATE OF tenant_id, election_event_id, action, scope_key, status, executed_at, subject OR DELETE
ON sequent_backend.signing_request
FOR EACH ROW EXECUTE FUNCTION sequent_backend.update_signed_voting_approval();

CREATE FUNCTION sequent_backend.update_signed_voting_election()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP <> 'INSERT' THEN
        PERFORM sequent_backend.refresh_signed_voting_boundary(OLD.tenant_id, OLD.election_event_id);
    END IF;
    IF TG_OP <> 'DELETE' AND (TG_OP = 'INSERT'
       OR ROW(OLD.tenant_id, OLD.election_event_id) IS DISTINCT FROM ROW(NEW.tenant_id, NEW.election_event_id)) THEN
        PERFORM sequent_backend.refresh_signed_voting_boundary(NEW.tenant_id, NEW.election_event_id);
    END IF;
    RETURN NULL;
END;
$$;
CREATE TRIGGER update_signed_voting_election
AFTER INSERT OR UPDATE OF tenant_id, election_event_id OR DELETE
ON sequent_backend.election
FOR EACH ROW EXECUTE FUNCTION sequent_backend.update_signed_voting_election();

DO $$
DECLARE scope record;
BEGIN
    FOR scope IN SELECT DISTINCT tenant_id, election_event_id FROM sequent_backend.election LOOP
        PERFORM sequent_backend.refresh_signed_voting_boundary(scope.tenant_id, scope.election_event_id);
    END LOOP;
END;
$$;

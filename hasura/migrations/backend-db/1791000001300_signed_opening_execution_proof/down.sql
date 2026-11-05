-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

CREATE OR REPLACE FUNCTION sequent_backend.refresh_signed_voting_boundary(
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

DROP INDEX sequent_backend.lifecycle_fired_signed_opening_proof;
ALTER TABLE sequent_backend.signed_voting_boundary DROP COLUMN fingerprint;
ALTER TABLE sequent_backend.lifecycle_fired DROP COLUMN executed_channels;

-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Create the schedule indexes within the migration transaction.
CREATE INDEX IF NOT EXISTS scheduled_event_active_scope_task_idx
    ON sequent_backend.scheduled_event (tenant_id, election_event_id, task_id)
    WHERE archived_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS scheduled_event_active_voting_task_idx
    ON sequent_backend.scheduled_event (tenant_id, election_event_id, task_id)
    WHERE archived_at IS NULL
      AND task_id ~ '^tenant_[0-9a-f-]{36}_event_[0-9a-f-]{36}_election_[0-9a-f-]{36}_(START|END)_VOTING_PERIOD$';

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_index
        WHERE indexrelid IN (
            'sequent_backend.scheduled_event_active_scope_task_idx'::regclass,
            'sequent_backend.scheduled_event_active_voting_task_idx'::regclass
        ) AND NOT indisvalid
    ) THEN
        RAISE EXCEPTION 'scheduled_event index is invalid; remove the invalid index before retrying this migration';
    END IF;
END;
$$;

-- Hasura and imports can write schedules directly. Validate just the reserved
-- voting tasks; execution bookkeeping requires no trigger or additional lock.
ALTER TABLE sequent_backend.scheduled_event
ADD CONSTRAINT scheduled_event_voting_period_valid CHECK (
    CASE WHEN archived_at IS NULL AND task_id ~ '^tenant_[0-9a-f-]{36}_event_[0-9a-f-]{36}_election_[0-9a-f-]{36}_(START|END)_VOTING_PERIOD$' THEN
        COALESCE(
            task_id IN (
                'tenant_' || tenant_id::text || '_event_' || election_event_id::text
                    || '_election_' || (event_payload ->> 'election_id') || '_START_VOTING_PERIOD',
                'tenant_' || tenant_id::text || '_event_' || election_event_id::text
                    || '_election_' || (event_payload ->> 'election_id') || '_END_VOTING_PERIOD'
            )
            AND event_payload = jsonb_build_object('election_id',
                substring(task_id FROM '_election_([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})_'))
            AND (cron_config IS NULL OR (
                jsonb_typeof(cron_config) = 'object'
                AND COALESCE(jsonb_typeof(cron_config -> 'cron'), 'null') IN ('string', 'null')
                AND COALESCE(jsonb_typeof(cron_config -> 'scheduled_date'), 'null') IN ('string', 'null')
                AND CASE WHEN cron_config ->> 'scheduled_date' IS NULL THEN true ELSE
                    (cron_config ->> 'scheduled_date') ~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}[Tt ]([01][0-9]|2[0-3]):[0-5][0-9]:([0-5][0-9]|60)([.][0-9]+)?([Zz]|[+-][0-9]{2}:[0-9]{2})$'
                    AND (cron_config ->> 'scheduled_date')::timestamptz IS NOT NULL
                END
            )), false
        )
    ELSE true END
);

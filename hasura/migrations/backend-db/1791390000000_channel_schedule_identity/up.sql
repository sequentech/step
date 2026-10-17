-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

ALTER TABLE sequent_backend.scheduled_event
ADD CONSTRAINT scheduled_event_channel_task_valid CHECK (
    CASE WHEN archived_at IS NULL AND task_id ~ '^tenant_[0-9a-f-]{36}_event_[0-9a-f-]{36}(_election_[0-9a-f-]{36})?_(START|END)_VOTING_PERIOD_channels_' THEN
        COALESCE(
            event_processor IN ('START_VOTING_PERIOD', 'END_VOTING_PERIOD')
            AND task_id = 'tenant_' || tenant_id::text || '_event_' || election_event_id::text
                || CASE WHEN event_payload ->> 'election_id' IS NULL THEN ''
                   ELSE '_election_' || (event_payload ->> 'election_id') END
                || '_' || event_processor || '_channels_' || concat_ws('_',
                    CASE WHEN event_payload -> 'voting_channels' ? 'KIOSK' THEN 'KIOSK' END,
                    CASE WHEN event_payload -> 'voting_channels' ? 'EARLY_VOTING' THEN 'EARLY_VOTING' END,
                    CASE WHEN event_payload -> 'voting_channels' ? 'TELEPHONE' THEN 'TELEPHONE' END)
            AND event_payload - 'election_id' - 'voting_channels' = '{}'::jsonb
            AND jsonb_typeof(event_payload -> 'voting_channels') = 'array'
            AND event_payload -> 'voting_channels' <> '[]'::jsonb
            AND event_payload -> 'voting_channels' <@ '["KIOSK", "EARLY_VOTING", "TELEPHONE"]'::jsonb
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

-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Supplemental display metadata only: never mutate already signed ballot EML.
SELECT election.id::text AS election_id,
       CASE WHEN authority.allows_live AND period.end_date IS NOT NULL
                 AND (retained.scheduled_at IS NULL OR period.end_date::timestamptz < retained.scheduled_at)
            THEN period.end_date ELSE retained.scheduled_date END AS scheduled_at,
       CASE WHEN authority.allows_live AND period.end_date IS NOT NULL
                 AND (retained.scheduled_at IS NULL OR period.end_date::timestamptz < retained.scheduled_at)
            THEN (
                SELECT schedule.cron_config->>'timezone'
                FROM sequent_backend.scheduled_event schedule
                WHERE schedule.tenant_id = election.tenant_id
                  AND schedule.election_event_id = election.election_event_id
                  AND schedule.archived_at IS NULL
                  AND schedule.task_id IN (
                      format('tenant_%s_event_%s_election_%s_END_VOTING_PERIOD', election.tenant_id, election.election_event_id, election.id),
                      format('tenant_%s_event_%s_END_VOTING_PERIOD', election.tenant_id, election.election_event_id)
                  )
                  AND schedule.cron_config->>'scheduled_date' = period.end_date
                  AND (sequent_backend.voting_window_election_id(schedule) = election.id
                       OR sequent_backend.voting_window_is_event_close(schedule))
                ORDER BY schedule.task_id = format('tenant_%s_event_%s_election_%s_END_VOTING_PERIOD', election.tenant_id, election.election_event_id, election.id) DESC
                LIMIT 1
            )
            ELSE retained.source_timezone END AS timezone
FROM sequent_backend.election election
LEFT JOIN sequent_backend.election_voting_window period
    ON period.tenant_id = election.tenant_id
   AND period.election_event_id = election.election_event_id
   AND period.election_id = election.id
LEFT JOIN LATERAL (
    SELECT closing.scheduled_date, closing.scheduled_at, closing.source_timezone
    FROM sequent_backend.signed_voting_boundary closing
    WHERE closing.tenant_id = election.tenant_id
      AND closing.election_event_id = election.election_event_id
      AND closing.election_id = election.id
      AND closing.event_processor = 'END_VOTING_PERIOD'
      AND closing.channels ? 'ONLINE'
      AND NOT EXISTS (
          SELECT 1 FROM sequent_backend.signed_voting_boundary opening
          WHERE opening.tenant_id = closing.tenant_id
            AND opening.election_event_id = closing.election_event_id
            AND opening.election_id = closing.election_id
            AND opening.event_processor = 'START_VOTING_PERIOD'
            AND opening.channels ? 'ONLINE'
            AND opening.scheduled_at > closing.scheduled_at
            AND opening.scheduled_at <= clock_timestamp()
      )
    ORDER BY closing.scheduled_at, closing.scheduled_event_id
    LIMIT 1
) retained ON true
CROSS JOIN LATERAL (
    SELECT sequent_backend.live_voting_close_allowed(election.tenant_id, election.election_event_id, election.id) AS allows_live
) authority
WHERE election.tenant_id = $1
  AND election.election_event_id = $2
  AND election.id = ANY($3::uuid[])

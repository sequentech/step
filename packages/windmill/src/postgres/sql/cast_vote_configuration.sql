-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Both inputs are maintained when configuration changes. No live schedule
-- scans or configuration-subject parsing are needed for a ballot.
SELECT election.presentation, election.status, election.voting_channels,
       period.start_date,
       CASE WHEN NOT sequent_backend.live_voting_close_allowed(election.tenant_id, election.election_event_id, election.id)
                THEN signed_bound.dates->>'ONLINE'
            WHEN signed_bound.dates->>'ONLINE' IS NULL THEN period.end_date
            WHEN period.end_date IS NULL
                 OR period.end_date::timestamptz > (signed_bound.dates->>'ONLINE')::timestamptz
                THEN signed_bound.dates->>'ONLINE'
            ELSE period.end_date
       END AS end_date,
       signed_bound.dates AS signed_close_dates
FROM sequent_backend.election AS election
LEFT JOIN sequent_backend.election_voting_window AS period
    ON period.tenant_id = election.tenant_id
   AND period.election_event_id = election.election_event_id
   AND period.election_id = election.id
LEFT JOIN LATERAL (
    SELECT COALESCE(jsonb_object_agg(channel, scheduled_date), '{}'::jsonb) AS dates
    FROM (
        SELECT DISTINCT ON (channel) channel, closing.scheduled_date
        FROM sequent_backend.signed_voting_boundary closing
        CROSS JOIN LATERAL jsonb_array_elements_text(closing.channels) channel
        WHERE closing.tenant_id = election.tenant_id
          AND closing.election_event_id = election.election_event_id
          AND closing.election_id = election.id
          AND closing.event_processor = 'END_VOTING_PERIOD'
          AND NOT EXISTS (
              SELECT 1 FROM sequent_backend.signed_voting_boundary opening
              WHERE opening.tenant_id = closing.tenant_id
                AND opening.election_event_id = closing.election_event_id
                AND opening.election_id = closing.election_id
                AND opening.event_processor = 'START_VOTING_PERIOD'
                AND opening.channels ? channel
                AND opening.scheduled_at > closing.scheduled_at
                AND opening.scheduled_at <= clock_timestamp()
                AND EXISTS (
                    SELECT 1 FROM sequent_backend.lifecycle_fired fired
                    WHERE fired.tenant_id = opening.tenant_id
                      AND fired.election_event_id = opening.election_event_id
                      AND fired.election_id = opening.election_id
                      AND fired.scheduled_event_id::text = opening.scheduled_event_id
                      AND fired.fingerprint = opening.fingerprint
                      AND fired.executed_channels ? channel
                      AND fired.fired_at >= opening.scheduled_at
                      AND fired.fired_at > closing.scheduled_at
                      AND fired.fired_at <= clock_timestamp()
                )
          )
        ORDER BY channel, closing.scheduled_at, closing.scheduled_event_id
    ) effective
) signed_bound ON true
WHERE election.tenant_id = $1
  AND election.election_event_id = $2
  AND election.id = $3

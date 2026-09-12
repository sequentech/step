-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

SELECT election.presentation, election.status, election.voting_channels,
       (
           SELECT schedule.cron_config ->> 'scheduled_date'
           FROM sequent_backend.scheduled_event AS schedule
           WHERE schedule.tenant_id = election.tenant_id
             AND schedule.election_event_id = election.election_event_id
             AND schedule.task_id = 'tenant_' || election.tenant_id::text
                 || '_event_' || election.election_event_id::text
                 || '_election_' || election.id::text || '_START_VOTING_PERIOD'
             AND schedule.event_payload = jsonb_build_object('election_id', election.id::text)
             AND schedule.archived_at IS NULL
       ) AS start_date,
       (
           SELECT schedule.cron_config ->> 'scheduled_date'
           FROM sequent_backend.scheduled_event AS schedule
           WHERE schedule.tenant_id = election.tenant_id
             AND schedule.election_event_id = election.election_event_id
             AND schedule.task_id = 'tenant_' || election.tenant_id::text
                 || '_event_' || election.election_event_id::text
                 || '_election_' || election.id::text || '_END_VOTING_PERIOD'
             AND schedule.event_payload = jsonb_build_object('election_id', election.id::text)
             AND schedule.archived_at IS NULL
       ) AS end_date
FROM sequent_backend.election AS election
WHERE election.tenant_id = $1
  AND election.election_event_id = $2
  AND election.id = $3

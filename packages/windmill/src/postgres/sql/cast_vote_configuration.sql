-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

SELECT election.presentation, election.status, election.voting_channels,
       period.start_date, period.end_date
FROM sequent_backend.election AS election
LEFT JOIN sequent_backend.election_voting_window AS period
    ON period.tenant_id = election.tenant_id
   AND period.election_event_id = election.election_event_id
   AND period.election_id = election.id
WHERE election.tenant_id = $1
  AND election.election_event_id = $2
  AND election.id = $3

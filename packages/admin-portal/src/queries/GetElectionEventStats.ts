// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_ELECTION_EVENT_STATS = gql`
    query GetElectionEventStats(
        $tenantId: uuid!
        $electionEventId: uuid!
        $startDate: String!
        $endDate: String!
        $userTimezone: String!
    ) {
        stats: getElectionEventStats(
            object: {
                election_event_id: $electionEventId
                start_date: $startDate
                end_date: $endDate
                user_timezone: $userTimezone
            }
        ) {
            total_eligible_voters
            total_distinct_voters
            total_areas
            total_elections
            votes_per_day {
                day
                day_count
            }
        }
        election_event: sequent_backend_election_event(
            where: {id: {_eq: $electionEventId}, tenant_id: {_eq: $tenantId}}
        ) {
            statistics
        }
    }
`

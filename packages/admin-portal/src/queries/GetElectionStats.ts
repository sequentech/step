// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_ELECTION_STATS = gql`
    query GetElectionStats(
        $tenantId: uuid!
        $electionEventId: uuid!
        $electionId: uuid!
        $startDate: String!
        $endDate: String!
        $electionAlias: String
        $userTimezone: String!
    ) {
        stats: getElectionStats(
            object: {
                election_event_id: $electionEventId
                election_id: $electionId
                start_date: $startDate
                end_date: $endDate
                user_timezone: $userTimezone
            }
        ) {
            total_distinct_voters
            total_areas
            votes_per_day {
                day
                day_count
            }
        }
        users: count_users(
            body: {
                tenant_id: $tenantId
                election_event_id: $electionEventId
                election_id: $electionId
                authorized_to_election_alias: $electionAlias
            }
        ) {
            count
        }
        election: sequent_backend_election(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                id: {_eq: $electionId}
            }
        ) {
            statistics
        }
    }
`

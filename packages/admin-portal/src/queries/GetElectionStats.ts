// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_ELECTION_STATS = gql`
    query GetElectionStats(
        $tenantId: uuid!,
        $electionEventId: uuid!,
        $electionId: uuid!
    ) {
        stats: getElectionStats(object: {
            election_event_id: $electionEventId,
            election_id: $electionId
        }) {
            total_distinct_voters
            total_areas
        }
        users: get_users(body: {
            tenant_id: $tenantId,
            election_event_id: $electionEventId,
            election_id: $electionId,
        }) {
            total {
                aggregate {
                    count
                }
            }
        }
    }
`

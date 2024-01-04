// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_ELECTION_EVENT_STATS = gql`
    query GetElectionEventStats($tenantId: uuid!, $electionEventId: uuid!) {
        castVotes: getElectionEventStats(object: {election_event_id: $electionEventId}) {
            total_distinct_voters
        }
        elections: sequent_backend_election_aggregate(
            where: {election_event_id: {_eq: $electionEventId}, tenant_id: {_eq: $tenantId}}
        ) {
            aggregate {
                count
            }
        }
        areas: sequent_backend_area_aggregate(
            where: {election_event_id: {_eq: $electionEventId}, tenant_id: {_eq: $tenantId}}
        ) {
            aggregate {
                count
            }
        }
        election_event: sequent_backend_election_event(
            where: {id: {_eq: $electionEventId}, tenant_id: {_eq: $tenantId}}
        ) {
            statistics
        }
        users: get_users(body: {tenant_id: $tenantId, election_event_id: $electionEventId}) {
            total {
                aggregate {
                    count
                }
            }
        }
    }
`

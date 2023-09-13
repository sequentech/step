// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_ELECTION_EVENT_STATS = gql`
    query GetElectionEventStats($electionEventId: uuid, $tenantId: uuid) {
        sequent_backend_cast_vote_aggregate(
            where: {election_event_id: {_eq: $electionEventId}, tenant_id: {_eq: $tenantId}}
        ) {
            aggregate {
                count
            }
        }
        sequent_backend_election_aggregate(
            where: {election_event_id: {_eq: $electionEventId}, tenant_id: {_eq: $tenantId}}
        ) {
            aggregate {
                count
            }
        }
        sequent_backend_area_aggregate(
            where: {election_event_id: {_eq: $electionEventId}, tenant_id: {_eq: $tenantId}}
        ) {
            aggregate {
                count
            }
        }
    }
`

// SPDX-FileCopyrightText: 2023 Eduardo Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_ELECTION_STATS = gql`
    query GetElectionStats($tenantId: uuid, $electionEventId: uuid, $electionId: uuid) {
        castVotes: sequent_backend_cast_vote_aggregate(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                election_id: {_eq: $electionId}
            }
        ) {
            aggregate {
                count
            }
        }
    }
`

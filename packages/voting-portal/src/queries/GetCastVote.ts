// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_CAST_VOTE = gql`
    query GetCastVote(
        $tenantId: uuid
        $electionEventId: uuid
        $electionId: uuid
        $ballotId: String!
    ) {
        sequent_backend_cast_vote(
            where: {
                _and: {
                    tenant_id: {_eq: $tenantId}
                    election_event_id: {_eq: $electionEventId}
                    election_id: {_eq: $electionId}
                    ballot_id: {_eq: $ballotId}
                }
            }
        ) {
            ballot_id
            content
        }
    }
`

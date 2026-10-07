// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_CAST_VOTE = gql`
    query GetCastVote(
        $electionEventId: String!
        $electionId: String!
        $ballotId: String
        $ballotIdPrefix: String
    ) {
        sequent_backend_cast_vote: get_voter_cast_votes(
            election_event_id: $electionEventId
            election_id: $electionId
            ballot_id: $ballotId
            ballot_id_prefix: $ballotIdPrefix
        ) {
            ballot_id
            content
        }
    }
`

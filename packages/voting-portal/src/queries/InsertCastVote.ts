// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const INSERT_CAST_VOTE = gql`
    mutation InsertCastVote(
        $electionId: uuid!
        $ballotId: String!
        $content: String!
    ) {
        insert_cast_vote(
            election_id: $electionId
            ballot_id: $ballotId
            content: $content
        ) {
            created_at
        }
    }
`

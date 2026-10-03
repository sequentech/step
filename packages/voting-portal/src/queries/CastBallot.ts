// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CAST_BALLOT = gql`
    mutation CastBallot($electionId: uuid!, $ballotId: String!, $castSignature: String!) {
        cast_ballot(
            election_id: $electionId
            ballot_id: $ballotId
            cast_signature: $castSignature
        ) {
            cast_vote_id
            cast_at
            key_id
            cast_receipt_signature
        }
    }
`

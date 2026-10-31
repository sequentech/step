// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const RECEIVE_BALLOT = gql`
    mutation ReceiveBallot($electionId: uuid!, $ballotId: String!, $content: String!) {
        receive_ballot(election_id: $electionId, ballot_id: $ballotId, content: $content) {
            ballot_id
            received_at
            key_id
            received_signature
        }
    }
`

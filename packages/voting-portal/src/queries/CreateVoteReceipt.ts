// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const CREATE_VOTE_RECEIPT = gql`
    mutation CreateVoteReceipt(
        $election_event_id: String!
        $tenant_id: String!
        $ballotId: String!
    ) {
        create_vote_receipt(
            election_event_id: $election_event_id
            tenant_id: $tenant_id
            ballot_id: $ballotId
        ) {
            id
            ballot_id
            status
        }
    }
`

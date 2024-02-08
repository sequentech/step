// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const CREATE_VOTE_RECEIPT = gql`
    mutation CreateVoteReceipt(
        $ballot_id: String!
        $ballot_tracker_url: String!
        $election_event_id: String!
        $tenant_id: String!
    ) {
        create_vote_receipt(
            ballot_id: $ballot_id
            ballot_tracker_url: $ballot_tracker_url
            election_event_id: $election_event_id
            tenant_id: $tenant_id
        ) {
            id
            ballot_id
            status
        }
    }
`

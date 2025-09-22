// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const CREATE_BALLOT_RECEIPT = gql`
    mutation createBallotReceipt(
        $ballot_id: String!
        $ballot_tracker_url: String!
        $election_event_id: uuid!
        $tenant_id: uuid!
        $election_id: uuid!
    ) {
        create_ballot_receipt(
            ballot_id: $ballot_id
            ballot_tracker_url: $ballot_tracker_url
            election_event_id: $election_event_id
            tenant_id: $tenant_id
            election_id: $election_id
        ) {
            id
            ballot_id
            status
        }
    }
`

// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import { gql } from "@apollo/client"

export const UPDATE_EVENT_VOTING_STATUS = gql`
    mutation UpdateTallyCeremony(
        $election_event_id: uuid!
        $status: String!
    ) {

    }
`

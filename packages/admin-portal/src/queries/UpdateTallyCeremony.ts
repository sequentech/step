// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPDATE_TALLY_CEREMONY = gql`
    mutation UpdateTallyCeremony(
        $election_event_id: uuid!
        $tally_session_id: uuid!
        $status: String!
    ) {
        update_tally_ceremony(
            election_event_id: $election_event_id
            tally_session_id: $tally_session_id
            status: $status
        ) {
            tally_session_id
        }
    }
`

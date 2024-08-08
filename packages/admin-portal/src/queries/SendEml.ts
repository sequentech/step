// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const SEND_EML = gql`
    mutation SendEml($electionEventId: uuid!, $tallySessionId: uuid!) {
        send_eml(
            election_event_id: $electionEventId
            tally_session_id: $tallySessionId
        ) {
            id
        }
    }
`

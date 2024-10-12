// SPDX-FileCopyrightText: 2024 Felix Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const LIST_KEYS_CEREMONY = gql`
    mutation ListKeysCeremony(
        $electionEventId: String!
    ) {
        list_keys_ceremony(
                election_event_id: $electionEventId
        ) {
            items
            count
        }
    }
`

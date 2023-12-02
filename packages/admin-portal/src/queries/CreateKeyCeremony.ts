// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_KEY_CEREMONY = gql`
    mutation CreateKeyCeremony(
        $electionEventId: String!
        $threshold: Int!
        $trusteeNames: [String!]
    ) {
        create_key_ceremony(
            object: {
                election_event_id: $electionEventId
                threshold: $threshold
                trustee_names: $trusteeNames
            }
        ) {
            key_ceremony_id
        }
    }
`

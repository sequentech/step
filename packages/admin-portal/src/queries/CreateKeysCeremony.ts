// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_KEYS_CEREMONY = gql`
    mutation CreateKeysCeremony(
        $electionEventId: String!
        $threshold: Int!
        $trusteeNames: [String!]
        $electionId: String
        $name: String
    ) {
        create_keys_ceremony(
            object: {
                election_event_id: $electionEventId
                threshold: $threshold
                trustee_names: $trusteeNames
                election_id: $electionId
                name: $name
            }
        ) {
            keys_ceremony_id
            error_message
        }
    }
`

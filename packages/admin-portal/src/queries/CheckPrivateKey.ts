// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CHECK_PRIVATE_KEY = gql`
    mutation CheckPrivateKey(
        $electionEventId: String!
        $keysCeremonyId: String!
        $privateKeyBase64: String!
    ) {
        check_private_key(
            object: {
                election_event_id: $electionEventId
                keys_ceremony_id: $keysCeremonyId
                private_key_base64: $privateKeyBase64
            }
        ) {
            is_valid
        }
    }
`

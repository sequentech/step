// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const RESTORE_PRIVATE_KEY = gql`
    mutation RestorePrivateKey(
        $electionEventId: String!
        $tallySessionId: String!
        $privateKeyBase64: String!
    ) {
        restore_private_key(
            object: {
                election_event_id: $electionEventId
                tally_session_id: $tallySessionId
                private_key_base64: $privateKeyBase64
            }
        ) {
            is_valid
        }
    }
`

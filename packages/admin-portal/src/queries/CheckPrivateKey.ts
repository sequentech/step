// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CHECK_PRIVATE_KEY = gql`
    mutation CheckPrivateKey(
        $electionEventId: String!
        $keysCeremonyId: String!
        $privateKeyBase64: String!
        $keyShareSha256: String
        $signingRequestId: uuid
    ) {
        check_private_key(
            object: {
                election_event_id: $electionEventId
                keys_ceremony_id: $keysCeremonyId
                private_key_base64: $privateKeyBase64
                key_share_sha256: $keyShareSha256
                signing_request_id: $signingRequestId
            }
        ) {
            is_valid
            signing_request {
                id
                code
                required
                expires_at
            }
        }
    }
`

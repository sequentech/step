// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const REGISTER_TRUSTEE_KEY = gql`
    mutation RegisterTrusteeKey(
        $publicKey: String!
        $electionEventId: String!
        $keysCeremonyId: String!
    ) {
        register_trustee_key(
            object: {
                public_key: $publicKey
                election_event_id: $electionEventId
                keys_ceremony_id: $keysCeremonyId
            }
        ) {
            success
        }
    }
`

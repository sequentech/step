// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_PRIVATE_KEY = gql`
    mutation GetPrivateKey($electionEventId: String!, $keysCeremonyId: String!) {
        get_private_key(
            object: {election_event_id: $electionEventId, keys_ceremony_id: $keysCeremonyId}
        ) {
            private_key_base64
        }
    }
`

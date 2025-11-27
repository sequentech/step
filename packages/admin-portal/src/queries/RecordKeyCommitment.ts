// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const RECORD_KEY_COMMITMENT = gql`
    mutation RecordKeyCommitment(
        $electionEventId: String!
        $trusteeName: String!
        $saltB64: String!
        $iterations: Int!
        $hashB64: String!
    ) {
        record_key_commitment(
            object: {
                election_event_id: $electionEventId
                trustee_name: $trusteeName
                salt_b64: $saltB64
                iterations: $iterations
                hash_b64: $hashB64
            }
        ) {
            success
        }
    }
`

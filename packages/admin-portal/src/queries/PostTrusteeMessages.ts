// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

// NOTE: This mutation expects the backend to expose a post_trustee_messages
// operation that accepts base64-encoded Borsh payloads representing
// individual GrpcB3Message values and posts them to B3.
export const POST_TRUSTEE_MESSAGES = gql`
    mutation PostTrusteeMessages(
        $electionEventId: String!
        $boardName: String!
        $messagesB64: [String!]!
    ) {
        post_trustee_messages(
            election_event_id: $electionEventId
            board_name: $boardName
            messages_b64: $messagesB64
        ) {
            ok
        }
    }
`

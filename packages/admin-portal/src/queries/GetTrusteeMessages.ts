// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

// NOTE: This query expects the backend to expose a get_trustee_messages
// operation returning Borsh-encoded GrpcB3Message batches as base64.
export const GET_TRUSTEE_MESSAGES = gql`
    query GetTrusteeMessages($electionEventId: String!, $boardName: String!, $sinceId: Int!) {
        get_trustee_messages(
            election_event_id: $electionEventId
            board_name: $boardName
            since_id: $sinceId
        ) {
            messages_b64
            last_id
        }
    }
`

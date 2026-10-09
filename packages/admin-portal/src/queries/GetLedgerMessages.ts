// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

import {ILedgerMessage} from "@/types/messaging"

// The voter's latest message that failed or whose delivery isn't confirmed.
export const GET_VOTER_LAST_FAILURE = gql`
    query GetVoterLastFailure($tenantId: uuid!, $electionEventId: uuid!, $voterId: String!) {
        sequent_backend_message(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                voter_id: {_eq: $voterId}
                direction: {_eq: "OUTBOUND"}
                state: {_in: ["FAILED", "UNKNOWN"]}
            }
            order_by: [{updated_at: desc}]
            limit: 1
        ) {
            id
            channel
            state
            attempt
            error
            created_at
            updated_at
            failed_at
        }
    }
`

// The current state of the messages a page of the Logs list records.
export const GET_LEDGER_MESSAGE_STATES = gql`
    query GetLedgerMessageStates($tenantId: uuid!, $ids: [uuid!]!) {
        sequent_backend_message(where: {tenant_id: {_eq: $tenantId}, id: {_in: $ids}}) {
            id
            channel
            state
            attempt
            error
            created_at
            updated_at
        }
    }
`

export interface ILedgerMessagesResult {
    sequent_backend_message: ILedgerMessage[]
}

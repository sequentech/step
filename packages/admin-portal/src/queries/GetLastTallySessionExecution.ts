// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_LAST_TALLY_SESSION_EXECUTION = gql`
    query GetLastTallySessionExecution($tallySessionId: uuid!, $tenantId: uuid!) {
        sequent_backend_tally_session_execution(
            where: {tally_session_id: {_eq: $tallySessionId}, tenant_id: {_eq: $tenantId}}
            order_by: {created_at: desc}
            limit: 1
        ) {
            id
            tenant_id
            election_event_id
            created_at
            last_updated_at
            labels
            annotations
            current_message_id
            tally_session_id
            session_ids
            status
            results_event_id
            documents
        }
    }
`

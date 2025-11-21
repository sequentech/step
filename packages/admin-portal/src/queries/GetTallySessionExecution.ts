// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_TALLY_SESSION_EXECUTION = gql`
    query GetTallySessionExecution(
        $tallySessionId: uuid!
        $tenantId: uuid!
        $resultsEventId: uuid!
    ) {
        sequent_backend_tally_session_execution(
            where: {
                tally_session_id: {_eq: $tallySessionId}
                tenant_id: {_eq: $tenantId}
                results_event_id: {_eq: $resultsEventId}
            }
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

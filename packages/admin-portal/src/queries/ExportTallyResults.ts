// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const EXPORT_TALLY_RESULTS = gql`
    mutation ExportTallyResults($electionEventId: String!, $tallySessionId: String!) {
        export_tally_results(
            election_event_id: $electionEventId
            tally_session_id: $tallySessionId
        ) {
            document_id
            task_execution {
                id
                name
                execution_status
                created_at
                start_at
                end_at
                logs
                annotations
                labels
                executed_by_user
                tenant_id
                election_event_id
                type
            }
            error_msg
        }
    }
`

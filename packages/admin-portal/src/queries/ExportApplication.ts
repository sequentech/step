// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const EXPORT_APPLICATION = gql`
    mutation ExportApplication($tenantId: String!, $electionEventId: String, $electionId: String) {
        export_application(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            election_id: $electionId
        ) {
            error_msg
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
        }
    }
`

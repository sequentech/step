// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const EXPORT_TASKS_EXECUTION = gql`
    mutation ExportTasksExecution($tenantId: String!, $electionEventId: String!) {
        export_tasks_execution(tenant_id: $tenantId, election_event_id: $electionEventId) {
            error_msg
            document_id
        }
    }
`

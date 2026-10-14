// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const EXPORT_TENANT_LOGS = gql`
    mutation ExportTenantLogs(
        $format: String!
        $scope: String!
        $username: String
        $createdFrom: String
        $createdTo: String
        $timeZone: String
    ) {
        export_tenant_logs(
            format: $format
            scope: $scope
            username: $username
            created_from: $createdFrom
            created_to: $createdTo
            time_zone: $timeZone
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
                type
            }
        }
    }
`

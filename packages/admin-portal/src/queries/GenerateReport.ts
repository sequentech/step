// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GENERATE_REPORT = gql`
    mutation GenerateReport(
        $reportId: String!
        $reportMode: String!
        $tenantId: String!
        $electionEventId: String
    ) {
        generate_report(
            report_id: $reportId
            report_mode: $reportMode
            tenant_id: $tenantId
            election_event_id: $electionEventId
        ) {
            document_id
            encryption_policy
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

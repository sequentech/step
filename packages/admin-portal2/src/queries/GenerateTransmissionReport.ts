// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GENERATE_TRANSMISSION_REPORT = gql`
    mutation generate_transmission_report(
        $tenantId: String!
        $electionEventId: String!
        $electionId: String
        $tallySessionId: String
    ) {
        generate_transmission_report(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            election_id: $electionId
            tally_session_id: $tallySessionId
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

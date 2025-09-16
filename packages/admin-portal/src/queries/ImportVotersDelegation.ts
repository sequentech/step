// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const IMPORT_VOTERS_DELEGATION = gql`
    mutation ImportVotersDelegation(
        $tenantId: String!
        $electionEventId: String!
        $documentId: String!
    ) {
        import_voters_delegation(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            document_id: $documentId
        ) {
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

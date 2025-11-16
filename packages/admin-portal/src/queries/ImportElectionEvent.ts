// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const IMPORT_ELECTION_EVENT = gql`
    mutation ImportElectionEvent(
        $tenantId: String!
        $documentId: String!
        $password: String
        $checkOnly: Boolean
        $sha256: String
    ) {
        import_election_event(
            tenant_id: $tenantId
            document_id: $documentId
            password: $password
            check_only: $checkOnly
            sha256: $sha256
        ) {
            id
            message
            error
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

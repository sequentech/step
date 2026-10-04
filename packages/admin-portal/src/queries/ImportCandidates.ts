// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const IMPORT_CANDIDTATES = gql`
    mutation ImportCandidates($documentId: String!, $electionEventId: String!, $sha256: String) {
        import_candidates(
            election_event_id: $electionEventId
            document_id: $documentId
            sha256: $sha256
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

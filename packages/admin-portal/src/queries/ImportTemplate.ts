// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const IMPORT_TEMPLATES = gql`
    mutation ImportTemplates($tenantId: String!, $documentId: String!, $sha256: String) {
        import_templates(tenant_id: $tenantId, document_id: $documentId, sha256: $sha256) {
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

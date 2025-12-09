// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const IMPORT_TENANT_CONFIG = gql`
    mutation ImportTenantConfig(
        $tenantId: String!
        $documentId: String!
        $importConfigurations: ImportOptions
        $sha256: String
    ) {
        import_tenant_config(
            tenant_id: $tenantId
            document_id: $documentId
            import_configurations: $importConfigurations
            sha256: $sha256
        ) {
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

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const IMPORT_TENANT_CONFIG = gql`
    mutation ImportTenantConfig(
        $tenantId: String!
        $documentId: String!
        $importConfigurations: ImportOptions
    ) {
        import_tenant_config(
            tenant_id: $tenantId
            document_id: $documentId
            import_configurations: $importConfigurations
        ) {
            id
            message
            error
        }
    }
`

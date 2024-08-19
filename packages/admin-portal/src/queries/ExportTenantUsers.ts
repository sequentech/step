// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const EXPORT_TENANT_USERS = gql`
    mutation ExportTenantUsers($tenantId: String!) {
        export_tenant_users(tenant_id: $tenantId) {
            document_id
            task_id
        }
    }
`

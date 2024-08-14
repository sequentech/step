// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const EXPORT_ALL_USERS = gql`
    mutation ExportAllUsers($tenantId: String!) {
        export_all_users(tenant_id: $tenantId) {
            document_id
            task_id
        }
    }
`

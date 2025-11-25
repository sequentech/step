// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const DELETE_ROLE = gql`
    mutation DeleteRole($tenantId: String!, $roleId: String!) {
        delete_role(tenant_id: $tenantId, role_id: $roleId) {
            id
        }
    }
`

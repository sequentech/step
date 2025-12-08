// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const DELETE_ROLE_PERMISSION = gql`
    mutation DeleteRolePermission($tenantId: String!, $roleId: String!, $permissionName: String!) {
        delete_role_permission(
            tenant_id: $tenantId
            role_id: $roleId
            permission_name: $permissionName
        ) {
            id
        }
    }
`

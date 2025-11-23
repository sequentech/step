// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const SET_ROLE_PERMISSION = gql`
    mutation SetRolePermission($tenantId: String!, $roleId: String!, $permissionName: String!) {
        set_role_permission(
            tenant_id: $tenantId
            role_id: $roleId
            permission_name: $permissionName
        ) {
            id
        }
    }
`

// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const DELETE_USER_ROLE = gql`
    mutation DeleteUserRole($tenantId: String!, $userId: String!, $roleId: String!) {
        delete_user_role(tenant_id: $tenantId, user_id: $userId, role_id: $roleId) {
            id
        }
    }
`

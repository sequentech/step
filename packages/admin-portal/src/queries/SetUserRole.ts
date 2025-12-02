// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const SET_USER_ROLE = gql`
    mutation SetUserRole($tenantId: String!, $userId: String!, $roleId: String!) {
        set_user_role(tenant_id: $tenantId, user_id: $userId, role_id: $roleId) {
            id
        }
    }
`

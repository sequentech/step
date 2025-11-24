// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_ROLE = gql`
    mutation CreateRole($tenantId: String!, $role: KeycloakRole2!) {
        create_role(tenant_id: $tenantId, role: $role) {
            id
            name
            permissions
            access
            attributes
            client_roles
        }
    }
`

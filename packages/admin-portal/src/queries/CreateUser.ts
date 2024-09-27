// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_USER = gql`
    mutation CreateUser(
        $tenantId: String!
        $electionEventId: String
        $user: KeycloakUser2!
        $userRolesIds: [String!]
    ) {
        create_user(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            user: $user
            user_roles_ids: $userRolesIds
        ) {
            id
            attributes
            email
            email_verified
            enabled
            first_name
            last_name
            username
        }
    }
`

// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const LIST_USER_ROLES = gql`
    query ListUserRoles($tenantId: String!, $userId: String!, $electionEventId: String) {
        list_user_roles(
            tenant_id: $tenantId
            user_id: $userId
            election_event_id: $electionEventId
        ) {
            id
            name
            permissions
            access
            attributes
            client_roles
        }
    }
`

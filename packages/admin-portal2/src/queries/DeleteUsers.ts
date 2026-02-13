// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const DELETE_USERS = gql`
    mutation DeleteUsers($tenantId: String!, $electionEventId: String, $usersId: [String!]!) {
        delete_users(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            users_id: $usersId
        ) {
            ids
        }
    }
`

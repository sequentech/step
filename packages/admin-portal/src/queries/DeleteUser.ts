// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const DELETE_USER = gql`
    mutation DeleteUser($tenantId: String!, $electionEventId: String, $userId: String!) {
        delete_user(tenant_id: $tenantId, election_event_id: $electionEventId, user_id: $userId) {
            id
        }
    }
`

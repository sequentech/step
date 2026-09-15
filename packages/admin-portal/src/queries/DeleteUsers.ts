// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

/**
 * `usersId` carries an explicit selection. `selectAll` instead deletes every
 * voter matching the filters, resolved server side, because the browser only
 * knows the page it has loaded.
 *
 * The filter variables are named to match `buildUserFilterPayload` in
 * GetUsers.ts exactly, so the delete and the list cannot drift apart. Any
 * filter the list applies but this omits would resolve to MORE voters than the
 * operator can see, and the delete is not reversible.
 */
export const DELETE_USERS = gql`
    mutation DeleteUsers(
        $tenantId: String!
        $electionEventId: String
        $electionId: String
        $usersId: [String!]
        $selectAll: Boolean
        $first_name: json
        $last_name: json
        $username: json
        $email: json
        $attributes: json
        $has_voted: Boolean
        $enabled: Boolean
        $email_verified: Boolean
        $authorized_to_election_alias: String
    ) {
        delete_users(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            election_id: $electionId
            users_id: $usersId
            select_all: $selectAll
            first_name: $first_name
            last_name: $last_name
            username: $username
            email: $email
            attributes: $attributes
            has_voted: $has_voted
            enabled: $enabled
            email_verified: $email_verified
            authorized_to_election_alias: $authorized_to_election_alias
        ) {
            ids
            error_msg
            task_execution {
                id
                name
                execution_status
                created_at
                start_at
                end_at
                logs
                annotations
                labels
                executed_by_user
                tenant_id
                election_event_id
                type
            }
        }
    }
`

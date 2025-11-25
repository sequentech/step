// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const EDIT_USER = gql`
    mutation EditUser($body: EditUsersInput!) {
        edit_user(body: $body) {
            attributes
            email
            email_verified
            enabled
            first_name
            groups
            id
            last_name
            username
        }
    }
`

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const DELETE_MESSAGING_ACCOUNT = gql`
    mutation DeleteMessagingAccount($id: uuid!) {
        delete_messaging_account(id: $id) {
            id
        }
    }
`

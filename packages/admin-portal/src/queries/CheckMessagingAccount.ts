// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CHECK_MESSAGING_ACCOUNT = gql`
    mutation CheckMessagingAccount($id: uuid!) {
        check_messaging_account(id: $id) {
            status
        }
    }
`

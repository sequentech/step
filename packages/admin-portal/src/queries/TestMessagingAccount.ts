// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const TEST_MESSAGING_ACCOUNT = gql`
    mutation TestMessagingAccount(
        $id: uuid!
        $purpose: String!
        $destination: String!
        $language: String
        $template: String
    ) {
        test_messaging_account(
            id: $id
            purpose: $purpose
            destination: $destination
            language: $language
            template: $template
        ) {
            message_id
            state
            reason
        }
    }
`

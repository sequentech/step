// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// Typed credentials go with the account in one transaction: an address change
// is refused unless its credentials come with it, and a new account is never
// left without the credentials it was saved with.

export const UPSERT_MESSAGING_ACCOUNT = gql`
    mutation UpsertMessagingAccount(
        $id: uuid
        $channel: String
        $name: String!
        $sender: jsonb!
        $limits: jsonb
        $providerApproval: String
        $isDefault: Boolean
        $credentials: jsonb
    ) {
        upsert_messaging_account(
            id: $id
            channel: $channel
            name: $name
            sender: $sender
            limits: $limits
            provider_approval: $providerApproval
            is_default: $isDefault
            credentials: $credentials
        ) {
            id
            callback_url
            callback_reason
        }
    }
`

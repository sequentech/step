// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPSERT_MESSAGING_ACCOUNT = gql`
    mutation UpsertMessagingAccount(
        $id: uuid
        $channel: String
        $name: String!
        $sender: jsonb!
        $limits: jsonb
        $providerApproval: String
        $readiness: String
        $isDefault: Boolean
    ) {
        upsert_messaging_account(
            id: $id
            channel: $channel
            name: $name
            sender: $sender
            limits: $limits
            provider_approval: $providerApproval
            readiness: $readiness
            is_default: $isDefault
        ) {
            id
        }
    }
`

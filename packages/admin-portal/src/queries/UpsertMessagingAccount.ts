// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPSERT_MESSAGING_ACCOUNT = gql`
    mutation UpsertMessagingAccount(
        $id: uuid
        $name: String!
        $sender: jsonb!
        $limits: jsonb
        $providerApproval: String
        $isDefault: Boolean
    ) {
        upsert_messaging_account(
            id: $id
            name: $name
            sender: $sender
            limits: $limits
            provider_approval: $providerApproval
            is_default: $isDefault
        ) {
            id
        }
    }
`

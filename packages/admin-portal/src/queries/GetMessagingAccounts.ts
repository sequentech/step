// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_MESSAGING_ACCOUNTS = gql`
    query GetMessagingAccounts($tenantId: uuid!) {
        sequent_backend_messaging_account(
            where: {tenant_id: {_eq: $tenantId}}
            order_by: [{channel: asc}, {name: asc}]
        ) {
            id
            tenant_id
            channel
            provider
            name
            sender
            credentials
            limits
            provider_approval
            readiness
            status
            webhook_key
            is_default
            created_at
            updated_at
        }
    }
`

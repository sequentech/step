// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// Settings > Messaging, with the messaging-account-read role: the credential
// records hold only when each credential was last replaced. The webhook key is
// never read: the callback URL comes from messaging_account_callback_url.
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
            is_default
            created_at
            updated_at
        }
    }
`

// Every other screen, with the messaging-config-read role: the safe columns only.
export const GET_MESSAGING_ACCOUNT_SUMMARIES = gql`
    query GetMessagingAccountSummaries($tenantId: uuid!) {
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
            limits
            provider_approval
            readiness
            status
            is_default
            created_at
            updated_at
        }
    }
`

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// The full callback URL of an account at the public webhook component, or why
// there is none (one reason, from the backend's callback_readiness).
export const MESSAGING_ACCOUNT_CALLBACK_URL = gql`
    query MessagingAccountCallbackUrl($accountId: uuid!) {
        messaging_account_callback_url(account_id: $accountId) {
            callback_url
            reason
        }
    }
`

export interface IMessagingAccountCallbackUrlResult {
    messaging_account_callback_url?: {
        callback_url?: string | null
        reason?: string | null
    } | null
}

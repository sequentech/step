// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// How many voters must connect Messenger again if the account's Page changes:
// those whose messenger-page is the current Page, in the events using it.
export const MESSAGING_ACCOUNT_CHANGE_IMPACT = gql`
    query MessagingAccountChangeImpact($accountId: uuid!) {
        messaging_account_change_impact(account_id: $accountId) {
            affected_voters
            events
        }
    }
`

export interface IMessagingAccountChangeImpactResult {
    messaging_account_change_impact?: {
        affected_voters: number
        events: number
    } | null
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

import {EMessageChannel} from "@/types/messaging"

// The tenant's messages of one month (YYYY-MM, UTC) by channel, account,
// destination country, billing category and billable; each message once.
export const GET_MESSAGING_MONTHLY_VOLUMES = gql`
    query GetMessagingMonthlyVolumes($month: String!, $electionEventId: uuid) {
        messaging_monthly_volumes(month: $month, election_event_id: $electionEventId) {
            month
            rows {
                channel
                account_id
                account_name
                destination_country
                billing_category
                billable
                messages
            }
        }
    }
`

export interface IMonthlyVolume {
    channel: EMessageChannel
    /** `null`: the deployment's own email or SMS sender. */
    account_id?: string | null
    /** `null` with an account id: the account was deleted since. */
    account_name?: string | null
    /** ISO 3166-1 alpha-2; older records may hold a calling code. */
    destination_country?: string | null
    billing_category?: string | null
    /** `null`: the provider reported nothing. */
    billable?: boolean | null
    messages: number
}

export interface IGetMessagingMonthlyVolumesResult {
    messaging_monthly_volumes?: {month: string; rows: IMonthlyVolume[]} | null
}

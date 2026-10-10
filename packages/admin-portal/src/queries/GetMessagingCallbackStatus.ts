// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

import {IAccountCallbackStatus} from "@/types/messaging"

// Per account, what stops its delivery reports (and Messenger's Get Started)
// from reaching Step: `callback_readiness`, read with messaging-config-read or
// messaging-account-read.
export const GET_MESSAGING_CALLBACK_STATUS = gql`
    query GetMessagingCallbackStatus {
        messaging_callback_status {
            accounts {
                account_id
                blockers
                reasons
            }
        }
    }
`

export interface IGetMessagingCallbackStatusResult {
    messaging_callback_status?: {accounts: IAccountCallbackStatus[]} | null
}

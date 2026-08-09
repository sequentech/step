// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_PHONE_BLACKLIST_ENTRY = gql`
    mutation CreatePhoneBlacklistEntry(
        $election_event_id: uuid!
        $phone_e164: String!
        $reason: String
    ) {
        create_phone_blacklist_entry(
            election_event_id: $election_event_id
            phone_e164: $phone_e164
            reason: $reason
        ) {
            id
        }
    }
`

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const DELETE_PHONE_BLACKLIST_ENTRY = gql`
    mutation DeletePhoneBlacklistEntry($id: uuid!, $election_event_id: uuid!) {
        delete_phone_blacklist_entry(id: $id, election_event_id: $election_event_id) {
            id
        }
    }
`

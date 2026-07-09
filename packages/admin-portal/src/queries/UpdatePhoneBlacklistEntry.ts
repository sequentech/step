// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPDATE_PHONE_BLACKLIST_ENTRY = gql`
    mutation UpdatePhoneBlacklistEntry($id: uuid!, $reason: String) {
        update_sequent_backend_phone_blacklist_by_pk(
            pk_columns: {id: $id}
            _set: {reason: $reason}
        ) {
            id
        }
    }
`

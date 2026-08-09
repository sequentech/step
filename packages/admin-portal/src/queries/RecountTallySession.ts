// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const RECOUNT_TALLY_SESSION = gql`
    mutation RecountTallySession($election_event_id: uuid!, $tally_session_id: uuid!) {
        recount_tally_session(
            election_event_id: $election_event_id
            tally_session_id: $tally_session_id
        ) {
            tally_session_id
        }
    }
`

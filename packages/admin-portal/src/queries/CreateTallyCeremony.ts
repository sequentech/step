// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_TALLY_CEREMONY = gql`
    mutation CreateTallyCeremony(
        $election_event_id: uuid!
        $election_ids: [uuid!]!
        $configuration: jsonb
        $tally_type: String
        $area_ids: [uuid!]
    ) {
        create_tally_ceremony(
            election_event_id: $election_event_id
            election_ids: $election_ids
            configuration: $configuration
            tally_type: $tally_type
            area_ids: $area_ids
        ) {
            tally_session_id
            signing_request {
                id
                code
                required
                expires_at
            }
        }
    }
`

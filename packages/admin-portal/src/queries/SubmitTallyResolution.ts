// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const SUBMIT_TALLY_RESOLUTION = gql`
    mutation SubmitTallyResolution(
        $election_event_id: uuid!
        $tally_session_id: uuid!
        $resolutions: [TallyResolutionInput!]!
    ) {
        submit_tally_resolution(
            election_event_id: $election_event_id
            tally_session_id: $tally_session_id
            resolutions: $resolutions
        ) {
            success
            tally_session_id
            resolved_count
        }
    }
`

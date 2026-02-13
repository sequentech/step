// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const PUBLISH_TALLY_SHEET = gql`
    mutation PublishTallySheet($electionEventId: uuid!, $tallySheetId: uuid!, $publish: Boolean!) {
        publish_tally_sheet(
            election_event_id: $electionEventId
            tally_sheet_id: $tallySheetId
            publish: $publish
        ) {
            tally_sheet_id
        }
    }
`

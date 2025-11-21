// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const ADD_TALLY_SHEET_VERSION = gql`
    mutation AddTallySheetVersion(
        $electionEventId: String!
        $tallySheetId: String!
        $oldVersion: Int!
    ) {
        add_tally_sheet_version(
            election_event_id: $electionEventId
            tally_sheet_id: $tallySheetId
            old_version: $oldVersion
        ) {
            tally_sheet_id
        }
    }
`

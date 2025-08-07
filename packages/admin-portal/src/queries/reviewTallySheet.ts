// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const REVIEW_TALLY_SHEET = gql`
    mutation ReviewtTallySheet(
        $electionEventId: String!
        $tallySheetId: String!
        $newStatus: String!
        $version: Int!
    ) {
        review_tally_sheet(
            election_event_id: $electionEventId
            tally_sheet_id: $tallySheetId
            new_status: $newStatus
            version: $version
        ) {
            id
            tenant_id
            election_event_id
            election_id
            contest_id
            area_id
            created_at
            last_updated_at
            labels
            annotations
            reviewed_at
            reviewed_by_user_id
            content
            channel
            deleted_at
            created_by_user_id
            status
            version
        }
    }
`

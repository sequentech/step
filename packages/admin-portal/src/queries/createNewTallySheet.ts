// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const CREATE_NEW_TALLY_SHEET = gql`
    mutation CreateNewtTallySheet(
        $electionEventId: String!
        $channel: String!
        $content: jsonb!
        $contestId: String!
        $areaId: String!
    ) {
        create_new_tally_sheet(
            election_event_id: $electionEventId
            channel: $channel
            content: $content
            contest_id: $contestId
            area_id: $areaId
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

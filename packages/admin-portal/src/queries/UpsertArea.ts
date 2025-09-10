// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPSERT_AREA = gql`
    mutation UpsertArea(
        $id: String
        $name: String!
        $description: String
        $presentation: jsonb
        $tenantId: String!
        $electionEventId: String!
        $parentId: String
        $areaContestsIds: [String]
        $annotations: jsonb
        $labels: jsonb
        $type: String
        $early_start: EarlyStartPolicy!
    ) {
        upsert_area(
            id: $id
            name: $name
            description: $description
            election_event_id: $electionEventId
            tenant_id: $tenantId
            parent_id: $parentId
            area_contest_ids: $areaContestsIds
            annotations: $annotations
            labels: $labels
            type: $type
            early_start: $early_start
        ) {
            id
        }
    }
`

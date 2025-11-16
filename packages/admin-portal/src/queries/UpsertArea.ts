// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
        $allow_early_voting: EarlyVotingPolicy
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
            allow_early_voting: $allow_early_voting
        ) {
            id
        }
    }
`

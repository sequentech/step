// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const SEARCH_APPLICATIONS = gql`
    query SearchApplications(
        $regularFilters: jsonb!
        $jsonbFilters: jsonb!
        $sortField: String!
        $sortOrder: String!
        $page: Int!
        $perPage: Int!
        $electionEventId: uuid!
        $permissionLabel: String
    ) {
        search_applications(
            election_event_id: $electionEventId
            permission_label: $permissionLabel
            regular_filters: $regularFilters
            jsonb_filters: $jsonbFilters
            sort_field: $sortField
            sort_order: $sortOrder
            page: $page
            per_page: $perPage
        ) {
            id
            applicant_id
            verification_type
            status
            created_at
            updated_at
            applicant_data
            annotations
            area_id
            election_event_id
            tenant_id
            labels
            permission_label
        }
    }
`

export const COUNT_APPLICATIONS = gql`
    query CountApplications(
        $regularFilters: jsonb!
        $jsonbFilters: jsonb!
        $electionEventId: uuid!
        $permissionLabel: String
    ) {
        count_applications(
            election_event_id: $electionEventId
            permission_label: $permissionLabel
            regular_filters: $regularFilters
            jsonb_filters: $jsonbFilters
        )
    }
`

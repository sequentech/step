// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const SEARCH_APPLICATIONS = gql`
    query SearchApplications(
        $jsonb_filters: jsonb!
        $regular_filters: jsonb!
        $election_event_id: uuid!
        $per_page: Int!
        $sort_field: String!
        $sort_order: String!
        $offset: Int!
        $permission_label: String
    ) {
        search_applications(
            args: {
                jsonb_filters: $jsonb_filters
                regular_filters: $regular_filters
                election_event_id: $election_event_id
                per_page: $per_page
                sort_field: $sort_field
                sort_order: $sort_order
                offset: $offset
                permission_label: $permission_label
            }
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
        $jsonb_filters: jsonb!
        $regular_filters: jsonb!
        $election_event_id: uuid!
        $permission_label: String
    ) {
        count_applications(
            args: {
                jsonb_filters: $jsonb_filters
                regular_filters: $regular_filters
                election_event_id: $election_event_id
                permission_label: $permission_label
            }
        ) {
            count
        }
    }
`

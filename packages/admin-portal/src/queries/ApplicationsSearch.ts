// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_APPLICATION_BY_ID = gql`
    query GetApplicationById($id: String!, $electionEventId: uuid!, $permissionLabel: String) {
        application: sequent_backend_search_applications_func(
            args: {
                p_election_event_id: $electionEventId
                permission_label: $permissionLabel
                regular_filters: {id: $id}
                jsonb_filters: {}
                sort_field: "id"
                sort_order: "ASC"
                per_page: 1
                offset_value: 0
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

export const SEARCH_APPLICATIONS = gql`
    query SearchApplications(
        $regularFilters: jsonb!
        $jsonbFilters: jsonb!
        $sortField: String!
        $sortOrder: String!
        $offsetValue: Int!
        $perPage: Int!
        $electionEventId: uuid!
        $permissionLabel: String
    ) {
        sequent_backend_search_applications_func(
            args: {
                p_election_event_id: $electionEventId
                per_page: $perPage
                permission_label: $permissionLabel
                regular_filters: $regularFilters
                sort_order: $sortOrder
                sort_field: $sortField
                jsonb_filters: $jsonbFilters
                offset_value: $offsetValue
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
        total: sequent_backend_search_applications_func_aggregate(
            args: {
                p_election_event_id: $electionEventId
                regular_filters: $regularFilters
                jsonb_filters: $jsonbFilters
                permission_label: $permissionLabel
            }
        ) {
            aggregate {
                count
            }
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
        count: sequent_backend_search_applications_func_aggregate(
            args: {
                p_election_event_id: $electionEventId
                permission_label: $permissionLabel
                regular_filters: $regularFilters
                jsonb_filters: $jsonbFilters
            }
        ) {
            aggregate {
                count
            }
        }
    }
`

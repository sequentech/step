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

export const GetApplications = (params: any) => {
    console.log("aa IN GetApplications", params)
    return SEARCH_APPLICATIONS
}

// Function to build variables for the SEARCH_APPLICATIONS query
export const buildApplicationsVariables = (params: any) => {
    const {
        filter = {},
        pagination = {page: 1, perPage: 20},
        sort = {field: "created_at", order: "DESC"},
    } = params

    // Extract JSONB filters
    const jsonbFilters: {[key: string]: string} = {}
    const regularFilters: {[key: string]: string} = {}

    // Check which filters are for JSONB fields and which are for regular columns
    Object.entries(filter).forEach(([key, value]) => {
        if (key.startsWith("applicant_data")) {
            // Extract the field name after 'applicant_data.'
            // const fieldName = key.split(".")[1]
            Object.keys(filter[key]).forEach((fieldKey) => {
                const newField = fieldKey
                const newValue = filter[key][newField]
                jsonbFilters[newField] = newValue["_ilike"]
            })
        } else if (["verification_type", "applicant_id", "id", "status"].includes(key)) {
            if (value && typeof value === "string" && value.trim() !== "") {
                regularFilters[key as string] = value
            }
        }
    })

    const offset: number | null =
        pagination?.page && pagination?.perPage ? (pagination.page - 1) * pagination.perPage : null
    const limit: number | null = pagination?.perPage ? pagination?.perPage : null
    const sortBy: string | null = sort.field || null
    const sortOrder: string | null = sort.order || "ASC"

    console.log("aa GetApplications jsonbFilters", jsonbFilters)
    console.log("aa GetApplications regularFilters", regularFilters)

    return {
        regularFilters,
        jsonbFilters,
        sortField: sortBy || "created_at",
        sortOrder: sortOrder || "ASC",
        offsetValue: offset || 0,
        perPage: limit || 20,
        electionEventId: filter.election_event_id,
        permissionLabel: filter.permission_label || null,
    }
}

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

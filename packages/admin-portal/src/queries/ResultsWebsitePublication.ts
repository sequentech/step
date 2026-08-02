// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"
import {
    EResultsPublicationStatus,
    EResultsRouteScope,
    EResultsWebsiteAccess,
    EResultsWebsiteStatus,
    EResultsWebsiteVisibilityScope,
} from "@sequentech/ui-core"

export interface PublishResultsWebsiteVariables {
    election_event_id: string
    tally_session_id: string
    tally_session_execution_id: string
    results_event_id: string
    route_scope: EResultsRouteScope
    route_election_id?: string | null
    election_ids: string[]
    contest_ids: string[]
    access: EResultsWebsiteAccess
    visibility_scope: EResultsWebsiteVisibilityScope
}

export interface PublishResultsWebsiteData {
    publishResultsWebsite: {
        publication_id: string
        task_execution_id: string
        publication_status: EResultsPublicationStatus
        error_msg?: string | null
    }
}

export interface RevokeResultsPublicationVariables {
    election_event_id: string
    publication_id: string
}

export interface RevokeResultsPublicationData {
    revokeResultsPublication: {
        publication_id: string
        publication_status: EResultsPublicationStatus
    }
}

export interface ConfigureResultsWebsitePolicyVariables {
    election_event_id: string
    status: EResultsWebsiteStatus
    access: EResultsWebsiteAccess
    visibility_scope: EResultsWebsiteVisibilityScope
}

export interface ConfigureResultsWebsitePolicyData {
    configureResultsWebsitePolicy: ConfigureResultsWebsitePolicyVariables
}

export const PUBLISH_RESULTS_WEBSITE = gql`
    mutation PublishResultsWebsite(
        $election_event_id: String!
        $tally_session_id: String!
        $tally_session_execution_id: String!
        $results_event_id: String!
        $route_scope: ResultsRouteScope!
        $route_election_id: String
        $election_ids: [String!]!
        $contest_ids: [String!]!
        $access: ResultsWebsiteAccess!
        $visibility_scope: ResultsWebsiteVisibilityScope!
    ) {
        publishResultsWebsite(
            election_event_id: $election_event_id
            tally_session_id: $tally_session_id
            tally_session_execution_id: $tally_session_execution_id
            results_event_id: $results_event_id
            route_scope: $route_scope
            route_election_id: $route_election_id
            election_ids: $election_ids
            contest_ids: $contest_ids
            access: $access
            visibility_scope: $visibility_scope
        ) {
            publication_id
            task_execution_id
            publication_status
            error_msg
        }
    }
`

export const CONFIGURE_RESULTS_WEBSITE_POLICY = gql`
    mutation ConfigureResultsWebsitePolicy(
        $election_event_id: String!
        $status: ResultsWebsiteStatus!
        $access: ResultsWebsiteAccess!
        $visibility_scope: ResultsWebsiteVisibilityScope!
    ) {
        configureResultsWebsitePolicy(
            election_event_id: $election_event_id
            status: $status
            access: $access
            visibility_scope: $visibility_scope
        ) {
            election_event_id
            status
            access
            visibility_scope
        }
    }
`

export const REVOKE_RESULTS_PUBLICATION = gql`
    mutation RevokeResultsPublication($election_event_id: String!, $publication_id: String!) {
        revokeResultsPublication(
            election_event_id: $election_event_id
            publication_id: $publication_id
        ) {
            publication_id
            publication_status
        }
    }
`

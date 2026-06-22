// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const PUBLISH_RESULTS_WEBSITE = gql`
    mutation PublishResultsWebsite(
        $election_event_id: String!
        $tally_session_id: String!
        $tally_session_execution_id: String!
        $results_event_id: String!
        $route_scope: String!
        $route_election_id: String
        $election_ids: [String!]!
        $contest_ids: [String!]!
        $access: String!
        $visibility_scope: String!
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

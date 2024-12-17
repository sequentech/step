// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const FETCH_ELECTION_EVENTS_TREE = gql`
    query election_events_tree($tenantId: uuid!, $isArchived: Boolean!) {
        sequent_backend_election_event(
            where: {is_archived: {_eq: $isArchived}, _and: {tenant_id: {_eq: $tenantId}}}
        ) {
            id
            name
            alias
            presentation
            is_archived
        }
    }
`

export const FETCH_ELECTIONS_TREE = gql`
    query election_tree($tenantId: uuid!, $electionEventId: uuid!) {
        sequent_backend_election(
            where: {election_event_id: {_eq: $electionEventId}, _and: {tenant_id: {_eq: $tenantId}}}
        ) {
            id
            name
            alias
            presentation
            election_event_id
        }
    }
`

export const FETCH_CONTEST_TREE = gql`
    query contest_tree($tenantId: uuid!, $electionId: uuid!) {
        sequent_backend_contest(
            where: {election_id: {_eq: $electionId}, _and: {tenant_id: {_eq: $tenantId}}}
        ) {
            id
            name
            alias
            presentation
            election_event_id
            election_id
        }
    }
`

export const FETCH_CANDIDATE_TREE = gql`
    query candidate_tree($tenantId: uuid!, $contestId: uuid!) {
        sequent_backend_candidate(
            where: {contest_id: {_eq: $contestId}, _and: {tenant_id: {_eq: $tenantId}}}
        ) {
            id
            name
            alias
            presentation
            election_event_id
            contest_id
        }
    }
`

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_INITIALIZATION_COUNTRIES = gql`
    query GetInitializationCountries($electionEventId: uuid!) {
        sequent_backend_area(
            where: {election_event_id: {_eq: $electionEventId}}
            order_by: [{name: asc}, {id: asc}]
        ) {
            id
            name
            parent_id
        }
        sequent_backend_area_contest(where: {election_event_id: {_eq: $electionEventId}}) {
            area_id
            contest_id
        }
    }
`

export const GET_INITIALIZATION_CONTESTS = gql`
    query GetInitializationContests($electionEventId: uuid!, $electionId: uuid!) {
        sequent_backend_contest(
            where: {election_event_id: {_eq: $electionEventId}, election_id: {_eq: $electionId}}
        ) {
            id
            election_id
        }
    }
`

export const GET_INITIALIZATION_STYLES = gql`
    query GetInitializationStyles($electionEventId: uuid!, $electionId: uuid!) {
        sequent_backend_ballot_style(
            where: {
                election_event_id: {_eq: $electionEventId}
                election_id: {_eq: $electionId}
                deleted_at: {_is_null: true}
                area_id: {_is_null: false}
                ballot_publication: {
                    _or: [{is_generated: {_eq: true}}, {published_at: {_is_null: false}}]
                }
            }
        ) {
            area_id
        }
    }
`

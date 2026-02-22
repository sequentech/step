// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_ELECTIONS_BY_EXTERNAL_ID = gql`
    query GetElectionsByExternalId($external_ids: [String!]!, $election_event_id: uuid) {
        sequent_backend_election(
            where: {
                alias: {_in: $external_ids}
                _and: [{election_event_id: {_eq: $election_event_id}}]
            }
        ) {
            id
            alias
            election_event_id
            presentation
        }
    }
`

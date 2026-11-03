// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_ELECTIONS_BY_EXTERNAL_ID = gql`
    query GetElectionsByExternalId(
        $external_ids: [String!]!
        $ids: [uuid!]!
        $election_event_id: uuid
    ) {
        sequent_backend_election(
            where: {
                _or: [{external_id: {_in: $external_ids}}, {id: {_in: $ids}}]
                _and: [{election_event_id: {_eq: $election_event_id}}]
            }
        ) {
            id
            external_id
            election_event_id
            presentation
        }
    }
`

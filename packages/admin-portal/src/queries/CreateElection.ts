// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_ELECTION = gql`
    mutation CreateElection($electionEventId: String!, $name: String!, $description: String) {
        create_election(
            election_event_id: $electionEventId
            name: $name
            description: $description
        ) {
            id
        }
    }
`

// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_ELECTION = gql`
    mutation CreateElection(
        $electionEventId: String!
        $externalId: String!
        $presentation: jsonb
        $description: String
    ) {
        create_election(
            election_event_id: $electionEventId
            external_id: $externalId
            presentation: $presentation
            description: $description
        ) {
            id
        }
    }
`

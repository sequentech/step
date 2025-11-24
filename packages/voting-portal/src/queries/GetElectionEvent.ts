// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_ELECTION_EVENT = gql`
    query GetElectionEvent($electionEventId: uuid!, $tenantId: uuid!) {
        sequent_backend_election_event(
            where: {_and: {id: {_eq: $electionEventId}, tenant_id: {_eq: $tenantId}}}
        ) {
            id
            presentation
            status
            description
        }
    }
`

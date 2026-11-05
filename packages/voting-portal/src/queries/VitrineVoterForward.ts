// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// Private events: Harvest forwards `POST voter/upgrade` and
// `POST voter/opening?hub=` to the ballots log, for the voter's own ballot
// only. The types are written by hand until the schema with this action is
// merged and codegen runs again.
export const VITRINE_VOTER_FORWARD = gql`
    mutation VitrineVoterForward(
        $electionEventId: uuid!
        $route: String!
        $body: String!
        $hub: String
    ) {
        vitrine_voter_forward(
            election_event_id: $electionEventId
            route: $route
            body: $body
            hub: $hub
        ) {
            status
            body
        }
    }
`

export interface VitrineVoterForwardMutationVariables {
    electionEventId: string
    route: string
    body: string
    hub?: string | null
}

export interface VitrineVoterForwardMutation {
    vitrine_voter_forward?: {
        status: number
        body?: string | null
    } | null
}

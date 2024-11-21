// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const DELETE_ELECTION_EVENT = gql`
    mutation DeleteElectionEvent($electionEventId: String!) {
        delete_election_event(election_event_id: $electionEventId) {
            id
        }
    }
`

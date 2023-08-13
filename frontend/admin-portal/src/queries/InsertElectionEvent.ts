// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const INSERT_ELECTION_EVENT = gql`
    mutation CreateElectionEvent(
        $electionEvent: sequent_backend_election_event_insert_input!,
        $elections: [sequent_backend_election_insert_input!]!
    ) {
        insert_sequent_backend_election_event(objects: [$electionEvent]) {
            returning {
                id
            }
        }
        insert_sequent_backend_election(objects: $elections) {
            returning {
                id
            }
        }
    }
`

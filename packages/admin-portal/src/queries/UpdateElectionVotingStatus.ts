// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPDATE_ELECTION_VOTING_STATUS = gql`
    mutation UpdateElectionVotingStatus(
        $electionId: uuid!
        $electionEventId: uuid!
        $status: String!
    ) {
        update_election_voting_status(
            election_id: $electionId
            voting_status: $status
            election_event_id: $electionEventId
        ) {
            election_id
        }
    }
`

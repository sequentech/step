// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPDATE_ELECTION_VOTING_STATUS = gql`
    mutation UpdateElectionVotingStatus(
        $election_event_id: uuid!
        $tally_session_id: uuid!
        $status: String!
    ) {
        
    }
`

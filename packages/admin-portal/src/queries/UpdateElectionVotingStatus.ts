// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPDATE_ELECTION_VOTING_STATUS = gql`
    mutation UpdateElectionVotingStatus(
        $electionId: uuid!
        $electionEventId: uuid!
        $votingStatus: VotingStatus!
        $votingChannel: [VotingStatusChannel]
    ) {
        update_election_voting_status(
            election_id: $electionId
            election_event_id: $electionEventId
            voting_status: $votingStatus
            voting_channels: $votingChannel
        ) {
            election_id
        }
    }
`

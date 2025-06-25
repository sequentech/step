// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const UPDATE_EVENT_VOTING_STATUS = gql`
    mutation UpdateEventVotingStatus(
        $electionEventId: uuid!
        $votingStatus: VotingStatus!
        $votingChannel: [VotingStatusChannel]
    ) {
        update_event_voting_status(
            election_event_id: $electionEventId
            voting_status: $votingStatus
            voting_channels: $votingChannel
        ) {
            election_event_id
        }
    }
`

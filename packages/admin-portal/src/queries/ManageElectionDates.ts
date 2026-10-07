// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MANAGE_ELECTION_DATES = gql`
    mutation ManageElectionDates(
        $electionEventId: uuid!
        $electionId: uuid
        $scheduledEventId: uuid
        $scheduledDate: String
        $localDateTime: String
        $timeZone: String
        $eventProcessor: String!
        $votingChannels: [VotingStatusChannel!]
    ) {
        manage_election_dates(
            election_event_id: $electionEventId
            election_id: $electionId
            scheduled_event_id: $scheduledEventId
            scheduled_date: $scheduledDate
            local_date_time: $localDateTime
            time_zone: $timeZone
            event_processor: $eventProcessor
            voting_channels: $votingChannels
        ) {
            error_msg
            scheduled_date
            warnings {
                code
                election_id
                message_key
                params
            }
        }
    }
`

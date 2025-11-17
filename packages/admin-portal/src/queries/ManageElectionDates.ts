// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MANAGE_ELECTION_DATES = gql`
    mutation ManageElectionDates(
        $electionEventId: String!
        $electionId: String
        $scheduledDate: String
        $eventProcessor: String!
    ) {
        manage_election_dates(
            election_event_id: $electionEventId
            election_id: $electionId
            scheduled_date: $scheduledDate
            event_processor: $eventProcessor
        ) {
            error_msg
        }
    }
`

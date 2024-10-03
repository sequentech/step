// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MANAGE_ELECTION_DATES = gql`
    mutation ManageElectionDates(
        $electionEventId: String!
        $electionId: String
        $start_date: String
        $end_date: String
    ) {
        manage_election_dates(
            election_event_id: $electionEventId
            election_id: $electionId
            start_date: $start_date
            end_date: $end_date
        ) {
            something
        }
    }
`

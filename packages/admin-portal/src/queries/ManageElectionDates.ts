// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MANAGE_ELECTION_DATES = gql`
    mutation ManageElectionDates(
        $electionEventId: String!
        $electionId: String!
        $isStart: Boolean!
        $isUnset: Boolean!
        $date: String
    ) {
        manage_election_dates(
            election_event_id: $electionEventId
            election_id: $electionId
            is_start: $isStart
            is_unset: $isUnset
            date: $date
        ) {
            something
        }
    }
`

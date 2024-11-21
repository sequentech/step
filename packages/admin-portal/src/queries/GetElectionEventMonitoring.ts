// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_ELECTION_EVENT_MONITORING = gql`
    query getElectionEventMonitoring($electionEventId: uuid!) {
        get_election_event_monitoring(election_event_id: $electionEventId) {
            total_eligible_voters
            total_enrolled_voters
            total_elections
            total_approved_voters
            total_disapproved_voters
            disapproved_resons
            total_open_votes
            total_not_opened_votes
            total_closed_votes
            total_not_closed_votes
            total_start_counting_votes
            total_not_start_counting_votes
            total_initialize
            total_not_initialize
            total_genereated_tally
            total_not_genereated_tally
            total_transmitted_results
            total_not_transmitted_results
        }
    }
`

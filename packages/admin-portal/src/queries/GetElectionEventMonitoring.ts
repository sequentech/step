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
            total_started_votes
            total_not_started_votes
            total_open_votes
            total_not_open_votes
            total_closed_votes
            total_not_closed_votes
            total_start_counting_votes
            total_not_start_counting_votes
            total_initialize
            total_not_initialize
            total_genereated_tally
            total_not_genereated_tally
            authentication_stats {
                total_authenticated
                total_not_authenticated
                total_invalid_users_errors
                total_invalid_password_errors
            }
            voting_stats {
                total_voted
                total_voted_tests_elections
            }
            approval_stats {
                total_approved
                total_disapproved
                total_manual_approved
                total_manual_disapproved
                total_automated_approved
                total_automated_disapproved
            }
            transmission_stats {
                total_transmitted_results
                total_half_transmitted_results
                total_not_transmitted_results
            }
        }
    }
`

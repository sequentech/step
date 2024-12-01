// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_ELECTION_MONITORING = gql`
    query getElectionMonitoring($electionEventId: uuid!, $electionId: uuid!) {
        get_election_monitoring(election_event_id: $electionEventId, election_id: $electionId) {
            total_eligible_voters
            total_enrolled_voters
            total_voted
            authentication_stats {
                total_authenticated
                total_not_authenticated
                total_invalid_users_errors
                total_invalid_password_errors
            }
            approval_stats {
                total_approved
                total_disapproved
                total_manual_approved
                total_manual_disapproved
                total_automated_approved
                total_automated_disapproved
            }
        }
    }
`

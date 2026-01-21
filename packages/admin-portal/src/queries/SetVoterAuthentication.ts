// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const SET_VOTER_AOTHENTICATION = gql`
    mutation SetVoterAuthentication(
        $electionEventId: String!
        $enrollment: String!
        $otp: String!
    ) {
        set_voter_authentication(
            election_event_id: $electionEventId
            enrollment: $enrollment
            otp: $otp
        ) {
            success
            message
        }
    }
`

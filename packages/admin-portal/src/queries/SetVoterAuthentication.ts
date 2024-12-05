// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const SET_VOTER_AOTHENTICATION = gql`
    mutation SetVoterAuthentication($enrollment: String!, $otp: String!) {
        set_voter_authentication(enrollment: $enrollment, otp: $otp) {
            success
            message
        }
    }
`

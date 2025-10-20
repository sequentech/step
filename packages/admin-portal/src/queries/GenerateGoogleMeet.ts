// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GENERATE_GOOGLE_MEET = gql`
    mutation GenerateGoogleMeet(
        $summary: String!
        $description: String!
        $startDateTime: String!
        $endDateTime: String!
        $timeZone: String!
        $attendeeEmails: [String!]!
    ) {
        generate_google_meet(
            summary: $summary
            description: $description
            start_date_time: $startDateTime
            end_date_time: $endDateTime
            time_zone: $timeZone
            attendee_emails: $attendeeEmails
        ) {
            meet_link
        }
    }
`

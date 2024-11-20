// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const DECRYPT_REPORT = gql`
    mutation DecryptReport($electionEventId: String!, $reportId: String, $password: String!) {
        decrypt_report(
            election_event_id: $electionEventId
            report_id: $reportId
            password: $password
        ) {
            document_id
            error_msg
        }
    }
`

// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const ENCRYPT_REPORT = gql`
    mutation EncryptReport($electionEventId: String!, $reportId: String, $password: String!) {
        encrypt_report(
            election_event_id: $electionEventId
            report_id: $reportId
            password: $password
        ) {
            document_id
            error_msg
        }
    }
`

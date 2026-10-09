// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const REPLACE_MESSAGING_ACCOUNT_CREDENTIALS = gql`
    mutation ReplaceMessagingAccountCredentials(
        $id: uuid!
        $credentials: jsonb!
        $generateVerifyToken: Boolean
        $generateReportCredentials: Boolean
    ) {
        replace_messaging_account_credentials(
            id: $id
            credentials: $credentials
            generate_verify_token: $generateVerifyToken
            generate_report_credentials: $generateReportCredentials
        ) {
            replaced
            verify_token
            report_username
            report_password
        }
    }
`

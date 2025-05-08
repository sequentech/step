// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const EXPORT_VERIFIABLE_BULLETIN_BOARD = gql`
    mutation ExportVerifiableBulletinBoard(
        $electionEventId: String!
        $tenantId: String!
        tallySessionId: String!
    ) {
        export_verifiable_bulletin_board(
            election_event_id: $electionEventId
            tenant_id: $tenantId
            tally_session_id: $tallySessionId
        ) {
            error_msg
            document_id
            task_execution {
                id
                name
                execution_status
                created_at
                start_at
                end_at
                logs
                annotations
                labels
                executed_by_user
                tenant_id
                election_event_id
                type
            }
        }
    }
`

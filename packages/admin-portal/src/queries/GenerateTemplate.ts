// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GENERATE_TEMPLATE = gql`
    mutation GenerateTemplate(
        $electionEventId: String!
        $electionId: String!
        $tallySessionId: String!
        $type: String!
    ) {
        generate_template(
            election_event_id: $electionEventId
            election_id: $electionId
            tally_session_id: $tallySessionId
            type: $type
        ) {
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

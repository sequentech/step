// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const EXPORT_USERS = gql`
    mutation ExportUsers($tenantId: String!, $electionEventId: String, $electionId: String) {
        export_users(
            tenant_id: $tenantId
            election_event_id: $electionEventId
            election_id: $electionId
        ) {
            document_id
            task_id
        }
    }
`

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CREATE_EXTERNAL_RECONCILIATION_IMPORT = gql`
    mutation CreateExternalReconciliationImport(
        $election_event_id: String!
        $document_id: String!
    ) {
        create_external_reconciliation_import(
            election_event_id: $election_event_id
            document_id: $document_id
        ) {
            task_execution {
                id
            }
        }
    }
`

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const APPLY_EXTERNAL_RECONCILIATION_CHANGES = gql`
    mutation ApplyExternalReconciliationChanges($election_event_id: String!, $diff_document_id: String!) {
        apply_external_reconciliation_changes(
            election_event_id: $election_event_id
            diff_document_id: $diff_document_id
        ) {
            task_execution {
                id
            }
        }
    }
`

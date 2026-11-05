// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const SAVE_APPROVAL_MATRIX = gql`
    mutation SaveApprovalMatrix($electionEventId: uuid!, $matrix: jsonb!) {
        save_approval_matrix(election_event_id: $electionEventId, matrix: $matrix) {
            version
            source
            matrix
            sha256
            created_at
            created_by
            next_version
            valid_ids
        }
    }
`

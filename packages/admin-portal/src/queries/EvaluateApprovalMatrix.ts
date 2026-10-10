// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const EVALUATE_APPROVAL_MATRIX = gql`
    query EvaluateApprovalMatrix($electionEventId: uuid!, $matrix: jsonb!, $enrollment: jsonb!) {
        evaluate_approval_matrix(
            election_event_id: $electionEventId
            matrix: $matrix
            enrollment: $enrollment
        ) {
            rule
            decision
            reason
            invariant
            errors {
                code
                rule
                field
            }
        }
    }
`

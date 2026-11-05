// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_APPROVAL_MATRIX = gql`
    query GetApprovalMatrix($electionEventId: uuid!) {
        get_approval_matrix(election_event_id: $electionEventId) {
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

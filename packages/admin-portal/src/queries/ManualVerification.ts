// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MANUAL_VERIFICATION = gql`
    mutation ManualVerification(
        $tenantId: String!
        $electionEventId: String!
        $voterId: String!
        $initiationId: String!
        $recoverRequestId: String
    ) {
        get_manual_verification_pdf(
            body: {
                tenant_id: $tenantId
                election_event_id: $electionEventId
                voter_id: $voterId
                initiation_id: $initiationId
                recover_request_id: $recoverRequestId
            }
        ) {
            document_id
            status
            signing_request
        }
    }
`

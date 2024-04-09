// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const MANUAL_VERIFICATION = gql`
    mutation ManualVerification($tenantId: String!, $electionEventId: String!, $voterId: String!) {
        get_manual_verification_pdf(
            body: {tenant_id: $tenantId, election_event_id: $electionEventId, voter_id: $voterId}
        ) {
            document_id
            status
        }
    }
`

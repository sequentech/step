// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const AUTHORIZE_TALLY_CUSTODY = gql`
    mutation AuthorizeTallyCustody(
        $electionEventId: String!
        $tallySessionId: String!
        $custodyProofJson: String!
        $custodyProofSha256: String
        $signingRequestId: uuid
    ) {
        authorize_tally_custody(
            object: {
                election_event_id: $electionEventId
                tally_session_id: $tallySessionId
                custody_proof_json: $custodyProofJson
                custody_proof_sha256: $custodyProofSha256
                signing_request_id: $signingRequestId
            }
        ) {
            is_valid
            signing_request {
                id
                code
                required
                expires_at
            }
        }
    }
`

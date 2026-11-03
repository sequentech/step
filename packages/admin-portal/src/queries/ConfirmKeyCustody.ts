// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const CONFIRM_KEY_CUSTODY = gql`
    mutation ConfirmKeyCustody(
        $electionEventId: String!
        $keysCeremonyId: String!
        $custodyProofJson: String!
        $custodyProofSha256: String
        $signingRequestId: uuid
    ) {
        confirm_key_custody(
            object: {
                election_event_id: $electionEventId
                keys_ceremony_id: $keysCeremonyId
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

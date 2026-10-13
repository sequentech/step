// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

/** Whether the signed-in trustee's key step needs their signature, and whether they have one. */
export interface ICustodyProofSignatureStatusQuery {
    custody_proof_signature_status?: {signature_needed: boolean; signed: boolean} | null
}

export interface ICustodyProofSignatureStatusVariables {
    electionEventId: string
    keysCeremonyId?: string | null
    tallySessionId?: string | null
}

export const CUSTODY_PROOF_SIGNATURE_STATUS = gql`
    query CustodyProofSignatureStatus(
        $electionEventId: String!
        $keysCeremonyId: String
        $tallySessionId: String
    ) {
        custody_proof_signature_status(
            object: {
                election_event_id: $electionEventId
                keys_ceremony_id: $keysCeremonyId
                tally_session_id: $tallySessionId
            }
        ) {
            signature_needed
            signed
        }
    }
`

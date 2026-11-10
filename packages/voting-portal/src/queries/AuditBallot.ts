// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const AUDIT_BALLOT = gql`
    mutation AuditBallot($electionId: uuid!, $ballotId: String!) {
        audit_ballot(election_id: $electionId, ballot_id: $ballotId) {
            ballot_id
            ballot_hash
        }
    }
`

// The ballot box's answer once it has marked the received ballot audited.
export interface AuditBallotMutation {
    audit_ballot?: {
        ballot_id: string
        ballot_hash: string
    } | null
}

export interface AuditBallotMutationVariables {
    electionId: string
    ballotId: string
}

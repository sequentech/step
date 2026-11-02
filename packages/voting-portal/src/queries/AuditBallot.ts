// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

/** Whether the ballot box published the audited ballot for anyone to decode. */
export enum EAuditPublication {
    PUBLISHED = "published",
    NOT_PUBLISHED = "not-published",
}

export const AUDIT_BALLOT = gql`
    mutation AuditBallot($electionId: uuid!, $ballotId: String!, $auditableBallot: String) {
        audit_ballot(
            election_id: $electionId
            ballot_id: $ballotId
            auditable_ballot: $auditableBallot
        ) {
            ballot_id
            status
            publication
            vitrine {
                kind
                text
                reason
            }
        }
    }
`

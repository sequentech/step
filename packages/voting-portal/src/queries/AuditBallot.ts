// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const AUDIT_BALLOT = gql`
    mutation AuditBallot($electionId: uuid!, $ballotId: String!) {
        audit_ballot(election_id: $electionId, ballot_id: $ballotId) {
            ballot_id
            status
        }
    }
`

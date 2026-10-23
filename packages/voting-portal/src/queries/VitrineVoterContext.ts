// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

// What the booth checks before it encrypts (step-integration.md, section 2.6).
// The types are written by hand until the schema with this action is merged
// and codegen runs again.
export const VITRINE_VOTER_CONTEXT = gql`
    query VitrineVoterContext($electionEventId: uuid!, $electionId: uuid!) {
        vitrine_voter_context(election_event_id: $electionEventId, election_id: $electionId) {
            mode
            policy
            policy_ack
            config
            opened
            style_hashes
            release
        }
    }
`

export interface VitrineVoterContextQueryVariables {
    electionEventId: string
    electionId: string
}

export interface VitrineVoterContextQuery {
    vitrine_voter_context?: {
        mode: string
        policy?: string | null
        policy_ack?: string | null
        /** jsonb: {statement, entry, chain} */
        config?: unknown
        /** jsonb: {statement, entry, chain} */
        opened?: unknown
        style_hashes?: string[] | null
        release?: string | null
    } | null
}

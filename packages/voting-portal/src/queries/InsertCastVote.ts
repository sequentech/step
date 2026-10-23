// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"
import type {InsertCastVoteMutation} from "../gql/graphql"
import type {IVitrineCast} from "../services/vitrine/types"

export const INSERT_CAST_VOTE = gql`
    mutation InsertCastVote($electionId: uuid!, $ballotId: String!, $content: String!) {
        insert_cast_vote(election_id: $electionId, ballot_id: $ballotId, content: $content) {
            id
            ballot_id
            election_id
            election_event_id
            tenant_id
            election_id
            area_id
            created_at
            last_updated_at
            labels
            annotations
            content
            cast_ballot_signature
            voter_id_string
            election_event_id
            vitrine {
                kind
                text
                reason
            }
        }
    }
`

/** The cast with Vitrine's answer (`InsertCastVoteOutput.vitrine`), until the
 * generated types have it. */
export type InsertCastVoteWithVitrineMutation = {
    insert_cast_vote?:
        | (NonNullable<InsertCastVoteMutation["insert_cast_vote"]> & {
              vitrine?: IVitrineCast | null
          })
        | null
}

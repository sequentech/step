// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {gql, TypedDocumentNode} from "@apollo/client"
import {GetVoterContextQuery, GetVoterContextQueryVariables} from "../gql/graphql"

// Hasura applies tenant, area and election claims to the ballot styles before
// traversing their scoped election relationship.
export const GET_VOTER_CONTEXT: TypedDocumentNode<
    GetVoterContextQuery,
    GetVoterContextQueryVariables
> = gql`
    query GetVoterContext($tenantId: uuid!, $electionEventId: uuid!) {
        sequent_backend_ballot_style(
            where: {
                tenant_id: {_eq: $tenantId}
                election_event_id: {_eq: $electionEventId}
                deleted_at: {_is_null: true}
            }
        ) {
            id
            election_id
            election_event_id
            status
            tenant_id
            ballot_eml
            ballot_signature
            created_at
            area_id
            annotations
            labels
            last_updated_at
            deleted_at
            election {
                annotations
                created_at
                description
                election_event_id
                id
                is_consolidated_ballot_encoding
                labels
                last_updated_at
                num_allowed_revotes
                presentation
                spoil_ballot_option
                status
                tenant_id
                voting_channels
            }
        }
        sequent_backend_election_event(
            where: {id: {_eq: $electionEventId}, tenant_id: {_eq: $tenantId}}
        ) {
            id
            presentation
            status
            description
        }
        sequent_backend_cast_vote(
            where: {tenant_id: {_eq: $tenantId}, election_event_id: {_eq: $electionEventId}}
        ) {
            id
            tenant_id
            election_id
            election_event_id
            status
        }
    }
`

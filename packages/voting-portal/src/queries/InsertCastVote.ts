// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const INSERT_CAST_VOTE = gql`
    mutation InsertCastVote(
        $id: uuid
        $ballotId: String!
        $electionId: uuid
        $electionEventId: uuid
        $tenantId: uuid
        $areaId: uuid
        $content: String!
    ) {
        insert_sequent_backend_cast_vote(
            objects: {
                id: $id
                ballot_id: $ballotId
                election_id: $electionId
                election_event_id: $electionEventId
                tenant_id: $tenantId
                area_id: $areaId
                content: $content
            }
        ) {
            returning {
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
            }
        }
    }
`

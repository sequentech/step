// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_CAST_VOTES = gql`
    query GetCastVotes {
        sequent_backend_cast_vote {
            id
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
`

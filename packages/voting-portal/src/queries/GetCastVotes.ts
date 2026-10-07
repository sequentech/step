// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_CAST_VOTES = gql`
    query GetCastVotes($electionEventId: String!) {
        cast_votes: get_voter_cast_votes(election_event_id: $electionEventId) {
            id
            tenant_id
            election_id
            election_event_id
            status
        }
    }
`

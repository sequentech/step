// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {gql, TypedDocumentNode} from "@apollo/client"
import {GetCastVotesQuery} from "../gql/graphql"
import {VoterFiles} from "../services/PublishedBallots"

export const GET_VOTER_STATUS: TypedDocumentNode<
    GetCastVotesQuery & {get_ballot_files_urls: VoterFiles},
    {electionEventId: string}
> = gql`
    query GetVoterStatus($electionEventId: String!) {
        get_ballot_files_urls(election_event_id: $electionEventId)
        sequent_backend_cast_vote {
            id
            tenant_id
            election_id
            election_event_id
            status
        }
    }
`

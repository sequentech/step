// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_BALLOT_PUBLICATION = gql`
    query GetBallotPublication($electionEventId: uuid!, $electionIds: [uuid!]) {
        sequent_backend_ballot_publication(
            where: {
                _and: {
                    election_event_id: {_eq: $electionEventId}
                    election_ids: {_contains: $electionIds}
                }
            }
        ) {
            annotations
            created_at
            created_by_user_id
            deleted_at
            election_event_id
            election_ids
            is_generated
            labels
            published_at
            tenant_id
            id
        }
    }
`

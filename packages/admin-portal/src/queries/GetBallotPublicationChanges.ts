// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_BALLOT_PUBLICATION_CHANGE = gql`
    mutation GetBallotPublicationChange(
        $ballotPublicationId: uuid!
        $electionEventId: uuid!
        $limit: Int
    ) {
        get_ballot_publication_changes(
            ballot_publication_id: $ballotPublicationId
            election_event_id: $electionEventId
            limit: $limit
        ) {
            current {
                ballot_publication_id
                ballot_styles
            }
            previous {
                ballot_publication_id
                ballot_styles
            }
        }
    }
`

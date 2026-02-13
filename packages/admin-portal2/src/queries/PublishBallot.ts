// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const PUBLISH_BALLOT = gql`
    mutation PublishBallot($electionEventId: uuid!, $ballotPublicationId: uuid!) {
        publish_ballot(
            election_event_id: $electionEventId
            ballot_publication_id: $ballotPublicationId
        ) {
            ballot_publication_id
        }
    }
`

// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_BALLOT_PUBLICATION_URL = gql`
    mutation GetBallotPublicationUrl($eventId: uuid!) {
        get_ballot_publication_url(election_event_id: $eventId) {
            url
        }
    }
`

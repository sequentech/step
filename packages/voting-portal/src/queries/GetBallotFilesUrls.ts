// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const GET_BALLOT_FILES_URLS = gql`
    mutation GetBallotFilesUrls($eventId: uuid!, $electionId: uuid!, $ballotPublicationId: uuid!) {
        get_ballot_files_urls(
            election_event_id: $eventId
            election_id: $electionId
            ballot_publication_id: $ballotPublicationId
        ) {
            urls
        }
    }
`

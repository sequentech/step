// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const FETCH_DOCUMENT = gql`
    query FetchDocument($electionEventId: String, $documentId: String!) {
        fetchDocument(election_event_id: $electionEventId, document_id: $documentId) {
            url
        }
    }
`

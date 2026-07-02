// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const IMPORT_AREAS = gql`
    mutation ImportAreas($electionEventId: String!, $documentId: String!, $sha256: String) {
        import_areas(
            election_event_id: $electionEventId
            document_id: $documentId
            sha256: $sha256
        ) {
            id
        }
    }
`

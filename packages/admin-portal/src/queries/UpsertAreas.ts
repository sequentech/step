// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const UPSERT_AREAS = gql`
    mutation UpsertAreas($electionEventId: String!, $documentId: String!) {
        upsert_areas(election_event_id: $electionEventId, document_id: $documentId) {
            id
        }
    }
`

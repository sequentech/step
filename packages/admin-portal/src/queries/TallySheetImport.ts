// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const PREVIEW_TALLY_SHEET_IMPORT = gql`
    mutation PreviewTallySheetImport(
        $electionEventId: String!
        $documentId: String!
        $sha256: String
        $sourceFormat: String!
        $selectedChannel: String!
    ) {
        preview_tally_sheet_import(
            election_event_id: $electionEventId
            document_id: $documentId
            sha256: $sha256
            source_format: $sourceFormat
            selected_channel: $selectedChannel
        ) {
            preview
        }
    }
`

export const CREATE_TALLY_SHEET_IMPORT = gql`
    mutation CreateTallySheetImport(
        $electionEventId: String!
        $documentId: String!
        $sha256: String
        $sourceFormat: String!
        $selectedChannel: String!
    ) {
        create_tally_sheet_import(
            election_event_id: $electionEventId
            document_id: $documentId
            sha256: $sha256
            source_format: $sourceFormat
            selected_channel: $selectedChannel
        ) {
            import
        }
    }
`

export const REVIEW_TALLY_SHEET_IMPORT = gql`
    mutation ReviewTallySheetImport(
        $electionEventId: String!
        $importId: String!
        $decision: String!
    ) {
        review_tally_sheet_import(
            election_event_id: $electionEventId
            import_id: $importId
            decision: $decision
        ) {
            import
        }
    }
`

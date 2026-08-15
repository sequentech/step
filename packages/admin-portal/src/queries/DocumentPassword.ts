// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {gql} from "@apollo/client"

export const GET_DOCUMENT_PASSWORD = gql`
    query GetDocumentPassword($documentId: String!) {
        get_document_password(document_id: $documentId) {
            password
        }
    }
`

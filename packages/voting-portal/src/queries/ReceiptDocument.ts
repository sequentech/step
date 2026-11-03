// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"

export const RECEIPT_DOCUMENT = gql`
    query ReceiptDocument($electionEventId: uuid!, $documentId: uuid!) {
        receipt_document(election_event_id: $electionEventId, document_id: $documentId) {
            payload
        }
    }
`
export interface IReceiptDocumentResponse {
    receipt_document: {payload: {document: {id: string; name: string | null; url: string} | null}}
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useQuery} from "@apollo/client/react"
import {RECEIPT_DOCUMENT, type IReceiptDocumentResponse} from "../queries/ReceiptDocument"

/** A receipt is a private capability: only its owner may poll or download it. */
export const useReceiptDocument = (electionEventId?: string, documentId?: string | null) =>
    useQuery<IReceiptDocumentResponse>(RECEIPT_DOCUMENT, {
        variables: {electionEventId, documentId},
        skip: !electionEventId || !documentId,
        fetchPolicy: "no-cache",
        context: {headers: {"x-hasura-role": "cast-vote"}},
    })

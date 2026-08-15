// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EExportFormat, IResultDocuments} from "@/types/results"

export const getExportDocumentId = (
    documents: IResultDocuments,
    format: EExportFormat
): string | undefined => {
    if (format === EExportFormat.TAR_GZ) {
        return documents.tar_gz_pdfs ?? documents.tar_gz
    }

    return documents[format]
}

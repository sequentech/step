// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

export enum EExportFormat {
    PDF = "pdf",
    JSON = "json",
    HTML = "html",
    TAR_GZ = "tar_gz",
    TAR_GZ_PDFS = "tar_gz_pdfs",
    RECEIPTS_PDF = "vote_receipts_pdf",
}

export type IResultDocuments = {
    [F in EExportFormat]?: string
}

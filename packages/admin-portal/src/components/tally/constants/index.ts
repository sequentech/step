// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EExportFormat} from "@/types/results"

export const EXPORT_FORMATS: Array<{
    label: string
    value: EExportFormat
}> = [
    {
        label: "PDF",
        value: EExportFormat.PDF,
    },
    {
        label: "HTML",
        value: EExportFormat.HTML,
    },
    {
        label: "JSON",
        value: EExportFormat.JSON,
    },
    {
        label: "TAR_GZ",
        value: EExportFormat.TAR_GZ,
    },
    {
        label: "TAR_GZ_PDFS",
        value: EExportFormat.TAR_GZ_PDFS,
    },
    {
        label: "RECEIPTS_PDF",
        value: EExportFormat.RECEIPTS_PDF,
    },
]

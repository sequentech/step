// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
]

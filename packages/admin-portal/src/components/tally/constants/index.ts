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
        label: "RECEIPTS_PDF",
        value: EExportFormat.RECEIPTS_PDF,
    },
]

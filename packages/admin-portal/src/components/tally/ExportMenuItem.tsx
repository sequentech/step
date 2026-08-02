// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, MenuItem} from "@mui/material"
import {EExportFormat, IResultDocuments} from "@/types/results"
import {IResultDocumentsData} from "./ExportElectionMenu"
import {useTranslation} from "react-i18next"

type ExportMenuItemProps = {
    documents: IResultDocumentsData
    className: string
    formatValue: EExportFormat
    formatLabel: string
    handleExport: (documents: IResultDocuments, format: EExportFormat) => void
    handleClose: () => void
    label?: string
}

export const ExportMenuItem = (props: ExportMenuItemProps) => {
    let {className, formatValue, handleExport, documents, formatLabel, handleClose, label} = props
    const {t} = useTranslation()

    return (
        <MenuItem
            className={className}
            key={formatValue}
            onClick={(e: React.MouseEvent<HTMLElement>) => {
                e.preventDefault()
                e.stopPropagation()
                setTimeout(() => handleClose(), 0)
                handleExport(documents.documents, formatValue)
            }}
        >
            <Box
                sx={{
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                    overflow: "hidden",
                }}
            >
                <span
                    title={
                        label ??
                        t("common.label.exportFormat", {
                            item: documents.name,
                            format: formatLabel,
                        })
                    }
                >
                    {label ??
                        t("common.label.exportFormat", {
                            item: documents.name,
                            format: formatLabel,
                        })}
                </span>
            </Box>
        </MenuItem>
    )
}

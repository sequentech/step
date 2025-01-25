// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo} from "react"
import {Box, MenuItem} from "@mui/material"
import {useTranslation} from "react-i18next"
import {ETemplateType} from "@/types/templates"

interface GenerateReportProps {
    electionId: string | null
    reportType: ETemplateType
}

export const GenerateReport: React.FC<GenerateReportProps> = ({electionId, reportType}) => {
    const {t} = useTranslation()

    return (
        <MenuItem
            onClick={(e: React.MouseEvent<HTMLElement>) => {
                e.preventDefault()
                e.stopPropagation()
            }}
        >
            <Box
                sx={{
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                    overflow: "hidden",
                }}
            >
                <span>
                    {t("tally.generateReport", {
                        name: t(`template.type.${reportType}`),
                    })}
                </span>
            </Box>
        </MenuItem>
    )
}

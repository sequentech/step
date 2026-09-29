// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {EColumnKind} from "./types"
import {formatInteger, formatRatio} from "./lib/format"

export interface MonitoringKpiProps {
    label: string
    value: unknown
    /** `integer` shows 874,624; `number` is a fraction shown as 53.2%. */
    kind: EColumnKind | string
}

/** One figure, large: a count grouped by the locale, a ratio as a percentage, "—" when undefined. */
export function MonitoringKpi({label, value, kind}: MonitoringKpiProps) {
    const {i18n} = useTranslation()
    const text =
        kind === EColumnKind.NUMBER
            ? formatRatio(value, i18n.language)
            : formatInteger(value, i18n.language)
    return (
        <Box sx={{minWidth: 120, px: 1}}>
            <Typography variant="h5" component="p" sx={{fontVariantNumeric: "tabular-nums"}}>
                {text}
            </Typography>
            <Typography variant="body2" color="text.secondary">
                {label}
            </Typography>
        </Box>
    )
}

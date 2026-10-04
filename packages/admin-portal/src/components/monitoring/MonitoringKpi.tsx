// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Typography} from "@mui/material"
import {useNumberFormat} from "@sequentech/ui-core"
import {EColumnKind} from "./types"
import {formatInteger, formatRatio} from "./lib/format"

export interface MonitoringKpiProps {
    label: string
    value: unknown
    /** `integer` shows 874,624; `number` is a fraction shown as 53.2%. */
    kind: EColumnKind | string
}

/**
 * One figure, large, in the election event's number format: a count, a ratio
 * as a percentage, "—" when undefined.
 */
export function MonitoringKpi({label, value, kind}: MonitoringKpiProps) {
    const {policy} = useNumberFormat()
    const text =
        kind === EColumnKind.NUMBER ? formatRatio(value, policy) : formatInteger(value, policy)
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

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Typography} from "@mui/material"
import type {IZonedFormat} from "@/hooks/useZonedFormat"

/**
 * A log time in the row's log zone, with "my time" below when the browser is
 * in another zone. `seconds` are Unix seconds, as the board stores them.
 */
export const LogTime: React.FC<{
    seconds: number | null | undefined
    zone: string
    format: IZonedFormat
}> = ({seconds, zone, format}) => {
    if (seconds === null || seconds === undefined) return <span>-</span>
    const instant = seconds * 1000
    const mine = format.mine(instant, zone)
    return (
        <Box component="span" sx={{display: "block", whiteSpace: "nowrap"}}>
            <span className="log-time">{format.format(instant, zone)}</span>
            {mine ? (
                <Typography
                    component="span"
                    variant="body2"
                    className="log-time-mine"
                    sx={{display: "block", color: "text.secondary"}}
                >
                    {mine}
                </Typography>
            ) : null}
        </Box>
    )
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Box, Typography} from "@mui/material"
import {useTimeZoneService, type IAdminTimeZones} from "./timeZoneService"

/** RFC 3339 needs an offset (`Z` or `±HH:MM`); a date without one never runs. */
export const hasOffset = (date: string | null | undefined): boolean =>
    !!date && /(?:[zZ]|[+-]\d{2}:?\d{2})$/.test(date.trim())

const parse = (instant: string | Date | null | undefined): Date | null => {
    if (!instant) return null
    const date = instant instanceof Date ? instant : new Date(instant)
    return Number.isNaN(date.getTime()) ? null : date
}

/** The two lines of an instant: in the row's zone, then in the viewer's ("my time"). */
export const zonedLines = (
    instant: Date,
    zone: string,
    service: IAdminTimeZones
): {primary: string; mine: string | null} => {
    const inZone = service.formatDateTimeZone(instant, zone, service.text)
    const inMine = service.formatDateTimeZone(instant, service.myTimeZone, service.text)
    // Both read the same: one line, marked as my time.
    return inZone === inMine
        ? {primary: service.formatMyTime(instant), mine: null}
        : {primary: inZone, mine: service.formatMyTime(instant)}
}

/**
 * An instant in the row's zone and in my time: "09 Apr 2028, 00:00 GMT+4"
 * over "08 Apr 2028, 16:00 EDT · my time", or one line when both are the same.
 */
export const ZonedDateTime: React.FC<{
    instant: string | Date | null | undefined
    zone: string
    /** Shown when there is no instant. */
    empty?: string
}> = ({instant, zone, empty = "-"}) => {
    const service = useTimeZoneService()
    const date = parse(instant)
    if (!date) return <>{empty}</>
    const {primary, mine} = zonedLines(date, zone, service)
    return (
        <Box component="span" sx={{display: "inline-flex", flexDirection: "column"}}>
            <Typography component="span" variant="body2">
                {primary}
            </Typography>
            {mine ? (
                <Typography component="span" variant="body2" color="text.secondary">
                    {mine}
                </Typography>
            ) : null}
        </Box>
    )
}

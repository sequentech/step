// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {styled} from "@mui/material/styles"
import {Box, Paper, Typography} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"

export const getToday: () => Date = () => {
    const date = new Date()
    const diffMinutes = date.getTimezoneOffset()
    date.setHours(-Math.floor(diffMinutes / 60), -diffMinutes % 60, 0, 0)
    return date
}

export function daysBefore(date: Date, days: number): Date {
    let before = new Date(date)
    before.setDate(before.getDate() - days)
    return before
}

export const formatDate: (date: Date) => String = (date: Date) => {
    return date.toISOString().substring(0, 4 + 3 + 3)
}

export function getWeekLegend(date: Date): Array<string> {
    const legend = ["M", "T", "W", "T", "F", "S", "S"]
    const dayOfWeek = date.getDay() // 0-6 day of week

    return [...legend.slice(dayOfWeek, 7), ...legend.slice(0, dayOfWeek)]
}

export const StyledPaper = styled(Paper)`
    padding: 16px;
`

export const Separator = styled(Box)`
    border-top: 1px solid ${theme.palette.customGrey.light};
    margin: 16px 0;
`

export default function CardChart({title, children}: {title: string; children: React.ReactNode}) {
    return (
        <StyledPaper>
            <Typography
                fontSize="16px"
                sx={{marginBottom: 0}}
                color={theme.palette.customGrey.main}
            >
                {title}
            </Typography>
            <Separator />
            {children}
        </StyledPaper>
    )
}

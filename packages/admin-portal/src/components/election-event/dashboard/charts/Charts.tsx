// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {styled} from "@mui/material/styles"
import {Box, Paper, Typography} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import {GetCastVotesQuery} from "@/gql/graphql"

const now = new Date()

export function daysBefore(date: Date, days: number): Date {
    let before = new Date(date)
    before.setDate(before.getDate() - days)
    return before
}

export function aggregateByDay(
    votes: GetCastVotesQuery["sequent_backend_cast_vote"]
): Array<number> {
    let values: Array<number> = []

    for (let i = 0; i < 7; i++) {
        let endDate = daysBefore(now, i)
        let startDate = daysBefore(now, i + 1)

        const filteredVotes = votes.filter((vote) => {
            let createdAt = new Date(vote.created_at)
            return createdAt < endDate && createdAt >= startDate
        })

        values.push(filteredVotes.length)
    }

    return values.reverse()
}

export function getWeekLegend(): Array<string> {
    const legend = ["M", "T", "W", "T", "F", "S", "S"]
    const dayOfWeek = now.getDay() // 0-6 day of week

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

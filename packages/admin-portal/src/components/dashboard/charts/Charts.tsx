// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useState} from "react"
import {styled} from "@mui/material/styles"
import {Box, Paper, Typography, Collapse, IconButton} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import {ExpandMore as ExpandMoreIcon} from "@mui/icons-material"

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

interface ExpandIconProps {
    expanded: boolean
}

export const ExpandIcon = styled(IconButton)<ExpandIconProps>`
    transform: ${(props) => (props.expanded ? "rotate(180deg)" : "rotate(0deg)")};
    transition: transform 0.3s;
    padding: 4px;
    margin-left: auto;
`

export default function CardChart({
    title,
    children,
    collapsible = false,
}: {
    title: string
    children: React.ReactNode
    collapsible?: boolean
}) {
    const [expanded, setExpanded] = useState(true)

    const handleExpandClick = () => {
        if (collapsible) {
            setExpanded(!expanded)
        }
    }

    return (
        <StyledPaper>
            <Box
                sx={{
                    display: "flex",
                    alignItems: "center",
                    cursor: collapsible ? "pointer" : "default",
                }}
                onClick={handleExpandClick}
            >
                <Typography
                    fontSize="16px"
                    sx={{marginBottom: 0}}
                    color={theme.palette.customGrey.main}
                >
                    {title}
                </Typography>
                {collapsible && (
                    <ExpandIcon expanded={expanded} size="small">
                        <ExpandMoreIcon />
                    </ExpandIcon>
                )}
            </Box>
            <Collapse in={expanded} timeout="auto" unmountOnExit>
                <Separator />
                {children}
            </Collapse>
        </StyledPaper>
    )
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo} from "react"
import {
    Box,
    Paper,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import type {ResultsAndParticipationLabels, ResultsParticipationSummary} from "./types"
import {mergeLabels, percentOrDash, toFiniteNumber} from "./utils"

interface ParticipationByChannelProps {
    result: ResultsParticipationSummary
    labels?: Partial<ResultsAndParticipationLabels>
}

const CHANNEL_ORDER = [
    "ONLINE",
    "KIOSK",
    "EARLY_VOTING",
    "TELEPHONE",
    "PAPER",
    "POSTAL",
    "IN_PERSON",
] as const

const channelOrder = new Map(CHANNEL_ORDER.map((channel, index) => [channel, index]))

const fallbackChannelLabel = (channel: string): string =>
    channel
        .toLowerCase()
        .split("_")
        .filter(Boolean)
        .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
        .join(" ") || channel

const channelLabel = (channel: string, labels: ResultsAndParticipationLabels): string => {
    const knownLabels: Record<string, string> = {
        ONLINE: labels.channelOnline,
        KIOSK: labels.channelKiosk,
        EARLY_VOTING: labels.channelEarlyVoting,
        TELEPHONE: labels.channelTelephone,
        PAPER: labels.channelPaper,
        POSTAL: labels.channelPostal,
        IN_PERSON: labels.channelInPerson,
    }
    return knownLabels[channel] ?? fallbackChannelLabel(channel)
}

export const ParticipationByChannel: React.FC<ParticipationByChannelProps> = ({result, labels}) => {
    const mergedLabels = useMemo(() => mergeLabels(labels), [labels])
    const eligibleCensus = toFiniteNumber(result.eligibleCensus)
    const rows = useMemo(
        () =>
            Object.entries(result.votesByChannel ?? {})
                .map(([channel, value]) => ({channel, total: toFiniteNumber(value)}))
                .filter(
                    (row): row is {channel: string; total: number} =>
                        row.total !== null && row.total > 0
                )
                .sort(
                    (left, right) =>
                        (channelOrder.get(left.channel) ?? Number.MAX_SAFE_INTEGER) -
                            (channelOrder.get(right.channel) ?? Number.MAX_SAFE_INTEGER) ||
                        left.channel.localeCompare(right.channel)
                ),
        [result.votesByChannel]
    )

    if (rows.length === 0) return null

    return (
        <Box className="seq-tally-results-participation-by-channel" sx={{mt: 4}}>
            <Typography component="h3" variant="h6" sx={{mb: 2, ml: 1}}>
                {mergedLabels.participationByChannel}
            </Typography>
            <TableContainer component={Paper} sx={{border: "1px solid", borderColor: "divider"}}>
                <Table sx={{minWidth: {xs: 300, sm: 500}, tableLayout: "fixed"}}>
                    <TableHead>
                        <TableRow>
                            <TableCell>{mergedLabels.channel}</TableCell>
                            <TableCell align="right">{mergedLabels.total}</TableCell>
                            <TableCell align="right">{mergedLabels.turnout}</TableCell>
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {rows.map(({channel, total}) => (
                            <TableRow key={channel}>
                                <TableCell component="th" scope="row">
                                    {channelLabel(channel, mergedLabels)}
                                </TableCell>
                                <TableCell align="right">{total}</TableCell>
                                <TableCell align="right">
                                    {percentOrDash(
                                        eligibleCensus && eligibleCensus > 0
                                            ? total / eligibleCensus
                                            : null
                                    )}
                                </TableCell>
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            </TableContainer>
        </Box>
    )
}

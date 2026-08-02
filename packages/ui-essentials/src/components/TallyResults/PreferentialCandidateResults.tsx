// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useEffect, useMemo, useState} from "react"
import {
    Box,
    Chip,
    IconButton,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    Paper,
    useMediaQuery,
    useTheme,
} from "@mui/material"
import {PREFERENTIAL_ROUND_COLUMN_WIDTH} from "./constants"
import type {
    CandidateReference,
    CandidateResultRow,
    PreferentialProcessResults,
    PreferentialRound,
    ResultsAndParticipationLabelOverrides,
    ResultsAndParticipationLabels,
} from "./types"
import {mergeLabels} from "./utils"

type CandidateStatus = "active" | "eliminated" | "winner"
type NavigationDirection = "left" | "right"

interface PreferentialCandidateResultsProps {
    processResults: PreferentialProcessResults
    candidates: CandidateResultRow[]
    labels?: ResultsAndParticipationLabelOverrides
}

interface RoundWindow {
    start: number
    end: number
}

const roundNavigationButtonSx = {
    "position": "absolute",
    "top": "50%",
    "transform": "translateY(-50%)",
    "border": 0,
    "borderRadius": "50%",
    "backgroundColor": "#1e3a5f",
    "color": "white",
    "width": 24,
    "height": 24,
    "&:hover": {
        backgroundColor: "#2c4f7c",
    },
} as const

const roundHeaderSx = {
    fontWeight: 600,
    width: PREFERENTIAL_ROUND_COLUMN_WIDTH,
    minWidth: PREFERENTIAL_ROUND_COLUMN_WIDTH,
    maxWidth: PREFERENTIAL_ROUND_COLUMN_WIDTH,
    whiteSpace: "nowrap",
    backgroundColor: "#FBFBFB",
    border: "1px solid #fff",
    position: "relative",
} as const

const candidateHeadingSx = {
    position: "sticky",
    left: 0,
    backgroundColor: "#FBFBFB",
    zIndex: 3,
    fontWeight: 600,
    border: "1px solid #fff",
} as const

const candidateCellSx = {
    position: "sticky",
    left: 0,
    backgroundColor: "#fff",
    zIndex: 2,
    border: "1px solid #fff",
    fontWeight: 500,
    maxWidth: 180,
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
} as const

const classToken = (value: string | number) => String(value).replace(/[^a-zA-Z0-9_-]/g, "-")

const roundOutcomeCellSx = (hasOutcome: boolean) => ({
    width: PREFERENTIAL_ROUND_COLUMN_WIDTH,
    minWidth: PREFERENTIAL_ROUND_COLUMN_WIDTH,
    maxWidth: PREFERENTIAL_ROUND_COLUMN_WIDTH,
    backgroundColor: hasOutcome ? "#F9F9FF" : "#E0E0E0",
    border: "1px solid #fff",
})

const getInitialRoundWindow = (visibleRoundsCount: number, roundCount: number): RoundWindow => ({
    start: 0,
    end: Math.min(visibleRoundsCount - 1, roundCount - 1),
})

const getCandidateStatusInRound = (
    rounds: PreferentialRound[],
    candidateId: string,
    roundIndex: number
): CandidateStatus => {
    const round = rounds[roundIndex]
    const isLastRound = roundIndex === rounds.length - 1

    if (round.winner?.id === candidateId) {
        return "winner"
    }

    if (isLastRound) {
        return "active"
    }

    if (round.eliminated_candidates?.some((candidate) => candidate.id === candidateId)) {
        return "eliminated"
    }

    for (let i = 0; i < roundIndex; i += 1) {
        if (rounds[i].eliminated_candidates?.some((candidate) => candidate.id === candidateId)) {
            return "eliminated"
        }
    }

    return "active"
}

const getCandidateName = (
    candidate: CandidateReference,
    candidateById: Map<string, CandidateResultRow>
) => candidateById.get(candidate.id)?.name ?? candidate.name

const RoundNavigationButton: React.FC<{
    direction: NavigationDirection
    ariaLabel: string
    onClick: () => void
}> = ({direction, ariaLabel, onClick}) => (
    <IconButton
        className={`seq-tally-results-preferential-results__${
            direction === "left" ? "previous" : "next"
        }-rounds-button`}
        size="small"
        onClick={onClick}
        aria-label={ariaLabel}
        sx={{
            ...roundNavigationButtonSx,
            ...(direction === "left" ? {left: 16} : {right: 16}),
        }}
    >
        {direction === "left" ? "<" : ">"}
    </IconButton>
)

const RoundStatusChip: React.FC<{
    status: CandidateStatus
    labels: ResultsAndParticipationLabels
}> = ({status, labels}) => {
    if (status === "winner") {
        return (
            <Chip
                className="seq-tally-results-preferential-results__winner-chip"
                label={labels.winner}
                sx={{
                    backgroundColor: "#4caf50",
                    color: "white",
                    fontWeight: 400,
                    fontSize: "0.875rem",
                }}
            />
        )
    }

    if (status === "eliminated") {
        return (
            <Chip
                className="seq-tally-results-preferential-results__eliminated-chip"
                label={labels.eliminated}
                variant="outlined"
                sx={{
                    borderColor: "#f44336",
                    color: "#f44336",
                    fontWeight: 400,
                    fontSize: "0.875rem",
                }}
            />
        )
    }

    return null
}

export const PreferentialCandidateResults: React.FC<PreferentialCandidateResultsProps> = ({
    processResults,
    candidates,
    labels,
}) => {
    const mergedLabels = useMemo(() => mergeLabels(labels), [labels])
    const theme = useTheme()
    const isXL = useMediaQuery(theme.breakpoints.up("xl"))
    const isLarge = useMediaQuery(theme.breakpoints.up("lg"))
    const visibleRoundsCount = isXL ? 3 : isLarge ? 2 : 1
    const roundCount = processResults.rounds.length
    const initialRoundWindow = useMemo(
        () => getInitialRoundWindow(visibleRoundsCount, roundCount),
        [roundCount, visibleRoundsCount]
    )
    const [representedRounds, setRepresentedRounds] = useState<RoundWindow>(initialRoundWindow)
    const candidateById = useMemo(() => new Map(candidates.map((c) => [c.id, c])), [candidates])

    useEffect(() => {
        setRepresentedRounds((current) => {
            if (
                current.start === initialRoundWindow.start &&
                current.end === initialRoundWindow.end
            ) {
                return current
            }

            return initialRoundWindow
        })
    }, [initialRoundWindow])

    const handleNavigate = useCallback(
        (direction: NavigationDirection) => {
            setRepresentedRounds((current) => {
                if (direction === "right" && current.end < processResults.rounds.length - 1) {
                    return {
                        start: current.start + 1,
                        end: current.end + 1,
                    }
                }

                if (direction === "left" && current.start > 0) {
                    return {
                        start: current.start - 1,
                        end: current.end - 1,
                    }
                }

                return current
            })
        },
        [processResults.rounds.length]
    )

    const handleKeyDown = useCallback(
        (event: React.KeyboardEvent) => {
            if (event.key === "ArrowLeft") {
                handleNavigate("left")
            } else if (event.key === "ArrowRight") {
                handleNavigate("right")
            } else if (event.key === "Home") {
                setRepresentedRounds(initialRoundWindow)
            } else if (event.key === "End") {
                setRepresentedRounds({
                    start: Math.max(0, processResults.rounds.length - visibleRoundsCount),
                    end: processResults.rounds.length - 1,
                })
            }
        },
        [handleNavigate, initialRoundWindow, processResults.rounds.length, visibleRoundsCount]
    )

    if (!processResults.rounds.length) {
        return null
    }

    const {rounds, name_references} = processResults
    const visibleRounds = rounds.slice(representedRounds.start, representedRounds.end + 1)

    return (
        <Box
            className="seq-tally-results-preferential-results"
            sx={{mt: 4, borderTop: "1px solid", borderColor: "divider", pt: 4}}
            tabIndex={0}
            onKeyDown={handleKeyDown}
        >
            <TableContainer
                className="seq-tally-results-preferential-results__table-container"
                component={Paper}
                sx={{
                    maxWidth: "100%",
                    boxShadow: "none",
                    border: "1px solid",
                    borderColor: "divider",
                }}
            >
                <Table className="seq-tally-results-preferential-results__table">
                    <TableHead className="seq-tally-results-preferential-results__table-head">
                        <TableRow className="seq-tally-results-preferential-results__heading-row">
                            <TableCell
                                className="seq-tally-results-preferential-results__candidate-heading"
                                sx={candidateHeadingSx}
                            >
                                {mergedLabels.candidate}
                            </TableCell>
                            {visibleRounds.map((_, visibleIndex) => {
                                const roundIndex = representedRounds.start + visibleIndex
                                const isFirstVisible = visibleIndex === 0
                                const isLastVisible = visibleIndex === visibleRounds.length - 1

                                return (
                                    <TableCell
                                        className={`seq-tally-results-preferential-results__round-heading seq-tally-results-preferential-results__round-heading--round-${
                                            roundIndex + 1
                                        }`}
                                        key={roundIndex}
                                        align="center"
                                        sx={roundHeaderSx}
                                    >
                                        {isFirstVisible && representedRounds.start > 0 && (
                                            <RoundNavigationButton
                                                direction="left"
                                                onClick={() => handleNavigate("left")}
                                                ariaLabel={mergedLabels.previousRounds}
                                            />
                                        )}
                                        <span className="seq-tally-results-preferential-results__round-label">
                                            {mergedLabels.round} {roundIndex + 1}
                                        </span>
                                        {isLastVisible &&
                                            representedRounds.end < rounds.length - 1 && (
                                                <RoundNavigationButton
                                                    direction="right"
                                                    onClick={() => handleNavigate("right")}
                                                    ariaLabel={mergedLabels.nextRounds}
                                                />
                                            )}
                                    </TableCell>
                                )
                            })}
                        </TableRow>
                    </TableHead>
                    <TableBody className="seq-tally-results-preferential-results__table-body">
                        {name_references.map((candidate) => {
                            const candidateName = getCandidateName(candidate, candidateById)

                            return (
                                <TableRow
                                    className={`seq-tally-results-preferential-results__row seq-tally-results-preferential-results__row--candidate-${classToken(
                                        candidate.id
                                    )}`}
                                    key={candidate.id}
                                >
                                    <TableCell
                                        className="seq-tally-results-preferential-results__candidate-cell"
                                        component="th"
                                        scope="row"
                                        sx={candidateCellSx}
                                        title={candidateName}
                                    >
                                        {candidateName}
                                    </TableCell>
                                    {visibleRounds.map((round, visibleIndex) => {
                                        const roundIndex = representedRounds.start + visibleIndex
                                        const status = getCandidateStatusInRound(
                                            rounds,
                                            candidate.id,
                                            roundIndex
                                        )
                                        const outcome = round.candidates_wins[candidate.id]

                                        return (
                                            <TableCell
                                                className={`seq-tally-results-preferential-results__round-cell seq-tally-results-preferential-results__round-cell--candidate-${classToken(
                                                    candidate.id
                                                )} seq-tally-results-preferential-results__round-cell--round-${
                                                    roundIndex + 1
                                                }`}
                                                key={roundIndex}
                                                align="center"
                                                sx={roundOutcomeCellSx(Boolean(outcome))}
                                            >
                                                {outcome ? (
                                                    <Stack
                                                        className="seq-tally-results-preferential-results__round-outcome"
                                                        direction="row"
                                                        alignItems="center"
                                                        justifyContent="flex-start"
                                                        spacing={2}
                                                    >
                                                        <Box
                                                            className="seq-tally-results-preferential-results__round-votes"
                                                            sx={{
                                                                color: "#333",
                                                                fontSize: "0.875rem",
                                                            }}
                                                        >
                                                            {outcome.wins.toLocaleString("en-US")} (
                                                            {(outcome.percentage * 100).toFixed(2)}
                                                            %)
                                                        </Box>
                                                        <RoundStatusChip
                                                            status={status}
                                                            labels={mergedLabels}
                                                        />
                                                    </Stack>
                                                ) : null}
                                            </TableCell>
                                        )
                                    })}
                                </TableRow>
                            )
                        })}
                    </TableBody>
                </Table>
            </TableContainer>
        </Box>
    )
}

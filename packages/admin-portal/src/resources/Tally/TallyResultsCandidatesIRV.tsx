// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useRef, useState, useEffect} from "react"
import {
    Box,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    IconButton,
    Chip,
    Paper,
    useTheme,
    useMediaQuery,
} from "@mui/material"
import {ChevronLeft, ChevronRight} from "@mui/icons-material"
import {RunoffStatus, ECandidateStatus} from "./types"
import {useTranslation} from "react-i18next"

interface TallyResultsCandidatesIRVProps {
    processResults: RunoffStatus
}

export const TallyResultsCandidatesIRV: React.FC<TallyResultsCandidatesIRVProps> = ({
    processResults,
}) => {
    const {t} = useTranslation()
    const theme = useTheme()
    const isXL = useMediaQuery(theme.breakpoints.up("xl"))
    const isLarge = useMediaQuery(theme.breakpoints.up("lg"))
    const isMedium = useMediaQuery(theme.breakpoints.up("md"))
    const VISIBLE_ROUNDS = isXL ? 4 : isLarge ? 3 : isMedium ? 2 : 1

    const [representedRounds, setRepresentedRounds] = useState({start: 0, end: VISIBLE_ROUNDS - 1})

    useEffect(() => {
        // Reset to initial range when data changes
        setRepresentedRounds({
            start: 0,
            end: Math.min(VISIBLE_ROUNDS - 1, processResults.rounds.length - 1),
        })
    }, [processResults, VISIBLE_ROUNDS])

    const handleNavigate = (direction: "left" | "right") => {
        const totalRounds = rounds.length

        if (direction === "right" && representedRounds.end < totalRounds - 1) {
            setRepresentedRounds({
                start: representedRounds.start + 1,
                end: representedRounds.end + 1,
            })
        } else if (direction === "left" && representedRounds.start > 0) {
            setRepresentedRounds({
                start: representedRounds.start - 1,
                end: representedRounds.end - 1,
            })
        }
    }

    const handleKeyDown = (e: React.KeyboardEvent) => {
        const totalRounds = rounds.length

        if (e.key === "ArrowLeft" && representedRounds.start > 0) {
            handleNavigate("left")
        } else if (e.key === "ArrowRight" && representedRounds.end < totalRounds - 1) {
            handleNavigate("right")
        } else if (e.key === "Home") {
            setRepresentedRounds({start: 0, end: Math.min(VISIBLE_ROUNDS - 1, totalRounds - 1)})
        } else if (e.key === "End") {
            setRepresentedRounds({
                start: Math.max(0, totalRounds - VISIBLE_ROUNDS),
                end: totalRounds - 1,
            })
        }
    }

    if (!processResults || !processResults.rounds || processResults.rounds.length === 0) {
        return null
    }

    const {rounds, name_references, candidates_status} = processResults

    // Get visible rounds based on current range
    const visibleRounds = rounds.slice(representedRounds.start, representedRounds.end + 1)

    // Calculate arrow visibility
    const showLeftArrow = representedRounds.start > 0
    const showRightArrow = representedRounds.end < rounds.length - 1

    // Format number with commas
    const formatNumber = (num: number): string => {
        return num.toLocaleString("en-US")
    }

    // Get candidate status for a specific round
    const getCandidateStatusInRound = (candidateId: string, roundIndex: number) => {
        const round = rounds[roundIndex]

        // Check if candidate is winner
        if (round.winner?.id === candidateId) {
            return "winner"
        }

        // Check if candidate was eliminated in this round
        if (round.eliminated_candidates?.some((c) => c.id === candidateId)) {
            return "eliminated"
        }

        // Check if candidate was eliminated in previous rounds
        for (let i = 0; i <= roundIndex; i++) {
            if (rounds[i].eliminated_candidates?.some((c) => c.id === candidateId)) {
                return "eliminated"
            }
        }

        return "active"
    }

    return (
        <Box
            sx={{
                mt: 4,
                borderTop: "1px solid #ccc",
                pt: 4,
            }}
            tabIndex={0}
            onKeyDown={handleKeyDown}
        >
            <TableContainer
                component={Paper}
                sx={{
                    maxWidth: "100%",
                    boxShadow: "none",
                    border: "1px solid #e0e0e0",
                }}
            >
                <Table>
                    <TableHead>
                        <TableRow>
                            <TableCell
                                sx={{
                                    position: "sticky",
                                    left: 0,
                                    backgroundColor: "#FBFBFB",
                                    zIndex: 3,
                                    fontWeight: 600,
                                    border: "1px solid #fff",
                                }}
                            >
                                {t("tally.table.preferential.candidate")}
                            </TableCell>
                            {visibleRounds.map((_, visibleIndex) => {
                                const roundIndex = representedRounds.start + visibleIndex
                                const isFirstVisible = visibleIndex === 0
                                const isLastVisible = visibleIndex === visibleRounds.length - 1

                                return (
                                    <TableCell
                                        key={roundIndex}
                                        align="center"
                                        sx={{
                                            fontWeight: 600,
                                            width: 320,
                                            minWidth: 320,
                                            maxWidth: 320,
                                            whiteSpace: "nowrap",
                                            backgroundColor: "#FBFBFB",
                                            border: "1px solid #fff",
                                            position: "relative",
                                        }}
                                    >
                                        {/* Left arrow - positioned at left margin */}
                                        {isFirstVisible && showLeftArrow && (
                                            <IconButton
                                                onClick={() => handleNavigate("left")}
                                                aria-label="Navigate to previous rounds"
                                                size="small"
                                                sx={{
                                                    "position": "absolute",
                                                    "left": 25,
                                                    "top": "50%",
                                                    "transform": "translateY(-50%)",
                                                    "backgroundColor": "#1e3a5f",
                                                    "color": "white",
                                                    "width": 24,
                                                    "height": 24,
                                                    "&:hover": {
                                                        backgroundColor: "#2c4f7c",
                                                    },
                                                }}
                                            >
                                                <ChevronLeft sx={{fontSize: 18}} />
                                            </IconButton>
                                        )}

                                        <span>
                                            {t("tally.table.preferential.round")} {roundIndex + 1}
                                        </span>

                                        {/* Right arrow - positioned at right margin */}
                                        {isLastVisible && showRightArrow && (
                                            <IconButton
                                                onClick={() => handleNavigate("right")}
                                                aria-label="Navigate to next rounds"
                                                size="small"
                                                sx={{
                                                    "position": "absolute",
                                                    "right": 25,
                                                    "top": "50%",
                                                    "transform": "translateY(-50%)",
                                                    "backgroundColor": "#1e3a5f",
                                                    "color": "white",
                                                    "width": 24,
                                                    "height": 24,
                                                    "&:hover": {
                                                        backgroundColor: "#2c4f7c",
                                                    },
                                                }}
                                            >
                                                <ChevronRight sx={{fontSize: 18}} />
                                            </IconButton>
                                        )}
                                    </TableCell>
                                )
                            })}
                        </TableRow>
                    </TableHead>
                    <TableBody>
                        {name_references.map((candidate, candidateIndex) => (
                            <TableRow key={candidate.id}>
                                <TableCell
                                    component="th"
                                    scope="row"
                                    sx={{
                                        "position": "sticky",
                                        "left": 0,
                                        "backgroundColor": "#fff",
                                        "zIndex": 2,
                                        "border": "1px solid #fff",
                                        "fontWeight": 500,
                                        "maxWidth": 180,
                                        "overflow": "hidden",
                                        "textOverflow": "ellipsis",
                                        "whiteSpace": "nowrap",
                                        "&:hover": {
                                            backgroundColor: "#f5f5f5",
                                        },
                                    }}
                                    title={candidate.name}
                                >
                                    {candidate.name}
                                </TableCell>
                                {visibleRounds.map((round, visibleIndex) => {
                                    const roundIndex = representedRounds.start + visibleIndex
                                    const status = getCandidateStatusInRound(
                                        candidate.id,
                                        roundIndex
                                    )
                                    const outcome = round.candidates_wins[candidate.id]

                                    return (
                                        <TableCell
                                            key={roundIndex}
                                            align="center"
                                            sx={{
                                                width: 320,
                                                minWidth: 320,
                                                maxWidth: 320,
                                                backgroundColor: outcome ? "#F9F9FF" : "#E0E0E0",
                                                border: "1px solid #fff",
                                            }}
                                        >
                                            {outcome ? (
                                                <Box
                                                    sx={{
                                                        display: "flex",
                                                        flexDirection: "row",
                                                        alignItems: "center",
                                                        justifyContent: "left",
                                                        gap: 5,
                                                    }}
                                                >
                                                    <Box
                                                        sx={{
                                                            color: "#333",
                                                            fontSize: "0.875rem",
                                                        }}
                                                    >
                                                        {formatNumber(outcome.wins)} (
                                                        {(outcome.percentage * 100).toFixed(2)}%)
                                                    </Box>
                                                    {status === "winner" && (
                                                        <Chip
                                                            label={t(
                                                                "tally.table.preferential.winner"
                                                            )}
                                                            sx={{
                                                                backgroundColor: "#4caf50",
                                                                color: "white",
                                                                fontWeight: 400,
                                                                fontSize: "0.875rem",
                                                            }}
                                                        />
                                                    )}
                                                    {status === "eliminated" && (
                                                        <Chip
                                                            label={t(
                                                                "tally.table.preferential.eliminated"
                                                            )}
                                                            variant="outlined"
                                                            sx={{
                                                                borderColor: "#f44336",
                                                                color: "#f44336",
                                                                fontWeight: 400,
                                                                fontSize: "0.875rem",
                                                            }}
                                                        />
                                                    )}
                                                </Box>
                                            ) : null}
                                        </TableCell>
                                    )
                                })}
                            </TableRow>
                        ))}
                    </TableBody>
                </Table>
            </TableContainer>
        </Box>
    )
}

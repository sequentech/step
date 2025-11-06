// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
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
} from "@mui/material"
import {ChevronLeft, ChevronRight} from "@mui/icons-material"
import {RunoffStatus, ECandidateStatus} from "./types"

interface TallyResultsCandidatesIRVProps {
    processResults: RunoffStatus
}

export const TallyResultsCandidatesIRV: React.FC<TallyResultsCandidatesIRVProps> = ({
    processResults,
}) => {
    const scrollContainerRef = useRef<HTMLDivElement>(null)
    const [showLeftArrow, setShowLeftArrow] = useState(false)
    const [showRightArrow, setShowRightArrow] = useState(false)

    useEffect(() => {
        console.log("TallyResultsCandidatesIRV processResults:", processResults)
    }, [processResults])

    useEffect(() => {
        updateArrowVisibility()
    }, [processResults])

    const updateArrowVisibility = () => {
        if (!scrollContainerRef.current) return

        const {scrollLeft, scrollWidth, clientWidth} = scrollContainerRef.current
        setShowLeftArrow(scrollLeft > 0)
        setShowRightArrow(scrollLeft + clientWidth < scrollWidth - 1)
    }

    const handleScroll = (direction: "left" | "right") => {
        if (!scrollContainerRef.current) return

        const scrollAmount = 200 // pixels to scroll
        const currentScroll = scrollContainerRef.current.scrollLeft
        const newScroll =
            direction === "left" ? currentScroll - scrollAmount : currentScroll + scrollAmount

        scrollContainerRef.current.scrollTo({
            left: newScroll,
            behavior: "smooth",
        })

        // Update arrow visibility after scroll
        setTimeout(updateArrowVisibility, 300)
    }

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === "ArrowLeft" && showLeftArrow) {
            handleScroll("left")
        } else if (e.key === "ArrowRight" && showRightArrow) {
            handleScroll("right")
        } else if (e.key === "Home" && scrollContainerRef.current) {
            scrollContainerRef.current.scrollTo({left: 0, behavior: "smooth"})
            setTimeout(updateArrowVisibility, 300)
        } else if (e.key === "End" && scrollContainerRef.current) {
            scrollContainerRef.current.scrollTo({
                left: scrollContainerRef.current.scrollWidth,
                behavior: "smooth",
            })
            setTimeout(updateArrowVisibility, 300)
        }
    }

    if (!processResults || !processResults.rounds || processResults.rounds.length === 0) {
        return null
    }

    const {rounds, name_references, candidates_status} = processResults

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
                position: "relative",
                mt: 4,
                borderTop: "1px solid #ccc",
                pt: 4,
            }}
            tabIndex={0}
            onKeyDown={handleKeyDown}
        >
            {/* Left Navigation Arrow */}
            {showLeftArrow && (
                <IconButton
                    onClick={() => handleScroll("left")}
                    aria-label="Scroll left to previous rounds"
                    sx={{
                        "position": "absolute",
                        "left": 140,
                        "top": "50%",
                        "transform": "translateY(-50%)",
                        "zIndex": 10,
                        "backgroundColor": "#1e3a5f",
                        "color": "white",
                        "width": 40,
                        "height": 40,
                        "&:hover": {
                            backgroundColor: "#2c4f7c",
                        },
                    }}
                >
                    <ChevronLeft />
                </IconButton>
            )}

            {/* Right Navigation Arrow */}
            {showRightArrow && (
                <IconButton
                    onClick={() => handleScroll("right")}
                    aria-label="Scroll right to next rounds"
                    sx={{
                        "position": "absolute",
                        "right": 16,
                        "top": "50%",
                        "transform": "translateY(-50%)",
                        "zIndex": 10,
                        "backgroundColor": "#1e3a5f",
                        "color": "white",
                        "width": 40,
                        "height": 40,
                        "&:hover": {
                            backgroundColor: "#2c4f7c",
                        },
                    }}
                >
                    <ChevronRight />
                </IconButton>
            )}

            <TableContainer
                component={Paper}
                ref={scrollContainerRef}
                onScroll={updateArrowVisibility}
                sx={{
                    "maxWidth": "100%",
                    "overflowX": "auto",
                    "boxShadow": "none",
                    "border": "1px solid #e0e0e0",
                    "&::-webkit-scrollbar": {
                        height: 8,
                    },
                    "&::-webkit-scrollbar-thumb": {
                        backgroundColor: "#ccc",
                        borderRadius: 4,
                    },
                }}
            >
                <Table sx={{minWidth: 650}}>
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
                                    minWidth: 180,
                                }}
                            >
                                Candidate
                            </TableCell>
                            {rounds.map((_, index) => (
                                <TableCell
                                    key={index}
                                    align="center"
                                    sx={{
                                        fontWeight: 600,
                                        minWidth: 150,
                                        whiteSpace: "nowrap",
                                        backgroundColor: "#FBFBFB",
                                        border: "1px solid #fff",
                                    }}
                                >
                                    Round {index + 1}
                                </TableCell>
                            ))}
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
                                {rounds.map((round, roundIndex) => {
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
                                                minWidth: 150,
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
                                                        justifyContent: "center",
                                                        gap: 2,
                                                    }}
                                                >
                                                    <Box
                                                        sx={{
                                                            color: "#333",
                                                            fontSize: "0.875rem",
                                                        }}
                                                    >
                                                        {formatNumber(outcome.wins)} (
                                                        {outcome.percentage.toFixed(2)}%)
                                                    </Box>
                                                    {status === "winner" && (
                                                        <Chip
                                                            label="Winner"
                                                            sx={{
                                                                backgroundColor: "#4caf50",
                                                                color: "white",
                                                                fontWeight: 500,
                                                                fontSize: "0.875rem",
                                                            }}
                                                        />
                                                    )}
                                                    {status === "eliminated" && (
                                                        <Chip
                                                            label="Eliminated"
                                                            variant="outlined"
                                                            sx={{
                                                                borderColor: "#f44336",
                                                                color: "#f44336",
                                                                fontWeight: 500,
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

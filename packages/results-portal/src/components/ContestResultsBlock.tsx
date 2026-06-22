// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo} from "react"
import {
    Box,
    Chip,
    Stack,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    Typography,
} from "@mui/material"
import {formatPercentOne, isNumber} from "@sequentech/ui-core"
import {ResultsManifestContest, ResultsRow, ResultsSqliteDataset} from "@/types/results"
import {translatedLabel} from "@/services/resultLabels"

interface ContestResultsBlockProps {
    manifestContest: ResultsManifestContest
    dataset: ResultsSqliteDataset
    locale: string
}

interface CandidateResult {
    id: string
    name: string
    castVotes?: number | null
    castVotesPercent?: number | null
    winningPosition?: number | null
}

const getNumber = (value: unknown): number | null =>
    typeof value === "number" && Number.isFinite(value) ? value : null

const sortCandidateResults = (left: CandidateResult, right: CandidateResult) => {
    const leftWinning = left.winningPosition ?? Number.MAX_SAFE_INTEGER
    const rightWinning = right.winningPosition ?? Number.MAX_SAFE_INTEGER

    if (leftWinning !== rightWinning) {
        return leftWinning - rightWinning
    }

    return (right.castVotes ?? 0) - (left.castVotes ?? 0)
}

export const ContestResultsBlock: React.FC<ContestResultsBlockProps> = ({
    manifestContest,
    dataset,
    locale,
}) => {
    const contest = dataset.contest.find((row) => row.id === manifestContest.contest_id)
    const election = dataset.election.find((row) => row.id === manifestContest.election_id)
    const resultContest = useMemo<ResultsRow | undefined>(() => {
        const source = manifestContest.area_id
            ? dataset.results_area_contest
            : dataset.results_contest

        return source.find(
            (row) =>
                row.contest_id === manifestContest.contest_id &&
                row.election_id === manifestContest.election_id &&
                (!manifestContest.area_id || row.area_id === manifestContest.area_id)
        )
    }, [dataset, manifestContest])

    const resultsCandidates = manifestContest.area_id
        ? dataset.results_area_contest_candidate
        : dataset.results_contest_candidate

    const candidates = useMemo<CandidateResult[]>(() => {
        return dataset.candidate
            .filter((candidate) => candidate.contest_id === manifestContest.contest_id)
            .map((candidate) => {
                const result = resultsCandidates.find(
                    (row) =>
                        row.candidate_id === candidate.id &&
                        row.contest_id === manifestContest.contest_id &&
                        row.election_id === manifestContest.election_id &&
                        (!manifestContest.area_id || row.area_id === manifestContest.area_id)
                )

                return {
                    id: candidate.id,
                    name: translatedLabel(candidate, locale),
                    castVotes: getNumber(result?.cast_votes),
                    castVotesPercent: getNumber(result?.cast_votes_percent),
                    winningPosition: getNumber(result?.winning_position),
                }
            })
            .sort(sortCandidateResults)
    }, [dataset.candidate, resultsCandidates, manifestContest, locale])

    const title =
        resultContest?.name ??
        translatedLabel(contest, locale, `Contest ${manifestContest.contest_id}`)
    const electionName = translatedLabel(election, locale, "Election")
    const isPublished = manifestContest.publication_state === "published"

    return (
        <Box
            component="section"
            sx={{
                mt: 3,
                p: {xs: 2, sm: 3},
                border: "1px solid",
                borderColor: "divider",
                borderRadius: 1,
                bgcolor: "background.paper",
            }}
        >
            <Stack
                direction={{xs: "column", sm: "row"}}
                spacing={1.5}
                justifyContent="space-between"
                alignItems={{xs: "flex-start", sm: "center"}}
            >
                <Box>
                    <Typography component="h3" variant="h6">
                        {title}
                    </Typography>
                    <Typography variant="body2" color="text.secondary">
                        {electionName}
                        {manifestContest.positions
                            ? ` - ${manifestContest.positions} position${
                                  manifestContest.positions === 1 ? "" : "s"
                              }`
                            : ""}
                    </Typography>
                </Box>
                <Chip
                    label={isPublished ? "Published" : "Not published yet"}
                    color={isPublished ? "success" : "default"}
                    variant={isPublished ? "filled" : "outlined"}
                    size="small"
                />
            </Stack>

            {!isPublished ? (
                <Typography sx={{mt: 3}} color="text.secondary">
                    Not published yet
                </Typography>
            ) : (
                <TableContainer sx={{mt: 3, overflowX: "auto"}}>
                    <Table aria-label={`${title} results`} sx={{minWidth: 620}}>
                        <TableHead>
                            <TableRow>
                                <TableCell>Options</TableCell>
                                <TableCell align="right">Number of votes</TableCell>
                                <TableCell align="right">Percent of votes</TableCell>
                                <TableCell align="right">Winning position</TableCell>
                            </TableRow>
                        </TableHead>
                        <TableBody>
                            {candidates.map((candidate) => (
                                <TableRow key={candidate.id}>
                                    <TableCell>{candidate.name}</TableCell>
                                    <TableCell align="right">
                                        {candidate.castVotes ?? "-"}
                                    </TableCell>
                                    <TableCell align="right">
                                        {isNumber(candidate.castVotesPercent)
                                            ? formatPercentOne(candidate.castVotesPercent)
                                            : "-"}
                                    </TableCell>
                                    <TableCell align="right">
                                        {candidate.winningPosition ?? "-"}
                                    </TableCell>
                                </TableRow>
                            ))}
                        </TableBody>
                    </Table>
                </TableContainer>
            )}
        </Box>
    )
}

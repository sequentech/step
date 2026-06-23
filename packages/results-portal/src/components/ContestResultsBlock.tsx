// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo} from "react"
import {Box, Chip, Stack, Typography} from "@mui/material"
import {ICountingAlgorithm} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"
import {
    CandidateResultRow,
    PreferentialProcessResults,
    ResultsAndParticipation,
    ResultsAndParticipationLabels,
    ResultsParticipationSummary,
} from "@sequentech/ui-essentials"
import {ResultsManifestContest, ResultsRow, ResultsSqliteDataset} from "@/types/results"
import {translatedLabel} from "@/services/resultLabels"

interface ContestResultsBlockProps {
    manifestContest: ResultsManifestContest
    dataset: ResultsSqliteDataset
    locale: string
}

const getNumber = (value: unknown): number | null => {
    if (typeof value === "number") {
        return Number.isFinite(value) ? value : null
    }

    if (typeof value === "string") {
        const parsed = Number(value.trim())
        return Number.isFinite(parsed) ? parsed : null
    }

    return null
}

const sameId = (left: unknown, right: unknown): boolean =>
    left !== null &&
    left !== undefined &&
    right !== null &&
    right !== undefined &&
    String(left) === String(right)

const parseAnnotations = (annotations: unknown): Record<string, any> | null => {
    if (!annotations) return null
    if (typeof annotations === "object") return annotations as Record<string, any>
    if (typeof annotations !== "string") return null

    try {
        return JSON.parse(annotations) as Record<string, any>
    } catch {
        return null
    }
}

export const ContestResultsBlock: React.FC<ContestResultsBlockProps> = ({
    manifestContest,
    dataset,
    locale,
}) => {
    const {t} = useTranslation()
    const contest = dataset.contest.find((row) => sameId(row.id, manifestContest.contest_id))
    const election = dataset.election.find((row) => sameId(row.id, manifestContest.election_id))
    const resultContest = useMemo<ResultsRow | undefined>(() => {
        const source = manifestContest.area_id
            ? dataset.results_area_contest
            : dataset.results_contest

        return source.find(
            (row) =>
                sameId(row.contest_id, manifestContest.contest_id) &&
                sameId(row.election_id, manifestContest.election_id) &&
                (!manifestContest.area_id || sameId(row.area_id, manifestContest.area_id))
        )
    }, [dataset, manifestContest])

    const resultsCandidates = manifestContest.area_id
        ? dataset.results_area_contest_candidate
        : dataset.results_contest_candidate

    const candidates = useMemo<CandidateResultRow[]>(() => {
        return dataset.candidate
            .filter((candidate) => sameId(candidate.contest_id, manifestContest.contest_id))
            .map((candidate) => {
                const result = resultsCandidates.find(
                    (row) =>
                        sameId(row.candidate_id, candidate.id) &&
                        sameId(row.contest_id, manifestContest.contest_id) &&
                        sameId(row.election_id, manifestContest.election_id) &&
                        (!manifestContest.area_id || sameId(row.area_id, manifestContest.area_id))
                )

                return {
                    id: String(candidate.id),
                    name: translatedLabel(candidate, locale),
                    castVotes: getNumber(result?.cast_votes),
                    castVotesPercent: getNumber(result?.cast_votes_percent),
                    winningPosition: getNumber(result?.winning_position),
                }
            })
    }, [dataset.candidate, resultsCandidates, manifestContest, locale])

    const summary = useMemo<ResultsParticipationSummary | null>(() => {
        if (!resultContest) return null

        return {
            id: resultContest.id,
            eligibleCensus: getNumber(resultContest.elegible_census),
            totalAuditableVotes: getNumber(resultContest.total_auditable_votes),
            totalAuditableVotesPercent: getNumber(resultContest.total_auditable_votes_percent),
            totalVotes: getNumber(resultContest.total_votes),
            totalVotesPercent: getNumber(resultContest.total_votes_percent),
            totalValidVotes: getNumber(resultContest.total_valid_votes),
            totalValidVotesPercent: getNumber(resultContest.total_valid_votes_percent),
            totalInvalidVotes: getNumber(resultContest.total_invalid_votes),
            totalInvalidVotesPercent: getNumber(resultContest.total_invalid_votes_percent),
            explicitInvalidVotes: getNumber(resultContest.explicit_invalid_votes),
            explicitInvalidVotesPercent: getNumber(resultContest.explicit_invalid_votes_percent),
            implicitInvalidVotes: getNumber(resultContest.implicit_invalid_votes),
            implicitInvalidVotesPercent: getNumber(resultContest.implicit_invalid_votes_percent),
            blankVotes: getNumber(resultContest.blank_votes),
            blankVotesPercent: getNumber(resultContest.blank_votes_percent),
            weight: getNumber(
                parseAnnotations(resultContest.annotations)?.extended_metrics?.weight
            ),
        }
    }, [resultContest])

    const processResults = useMemo<PreferentialProcessResults | null>(() => {
        if (contest?.counting_algorithm !== ICountingAlgorithm.INSTANT_RUNOFF) {
            return null
        }

        return (
            (parseAnnotations(resultContest?.annotations)?.process_results as
                | PreferentialProcessResults
                | undefined) ?? null
        )
    }, [contest?.counting_algorithm, resultContest?.annotations])

    const title =
        resultContest?.name ??
        translatedLabel(
            contest,
            locale,
            t("resultsPortal.fallbackContestName", {contestId: manifestContest.contest_id})
        )
    const electionName = translatedLabel(election, locale, t("resultsPortal.fallbackElectionName"))
    const area = dataset.area.find((row) => sameId(row.id, manifestContest.area_id))
    const areaName = manifestContest.area_id
        ? translatedLabel(area, locale, manifestContest.area_id)
        : null
    const chartName = areaName
        ? `${electionName} - ${title} - ${areaName}`
        : `${electionName} - ${title}`
    const isPublished = manifestContest.publication_state === "published"
    const labels = useMemo<Partial<ResultsAndParticipationLabels>>(
        () => ({
            participationSummary: t("resultsPortal.resultsAndParticipation.participationSummary"),
            candidateResults: t("resultsPortal.resultsAndParticipation.candidateResults"),
            total: t("resultsPortal.resultsAndParticipation.total"),
            turnout: t("resultsPortal.resultsAndParticipation.turnout"),
            eligibleCensus: t("resultsPortal.resultsAndParticipation.eligibleCensus"),
            totalAuditableVotes: t("resultsPortal.resultsAndParticipation.totalAuditableVotes"),
            totalVotesCounted: t("resultsPortal.resultsAndParticipation.totalVotesCounted"),
            totalValidVotes: t("resultsPortal.resultsAndParticipation.totalValidVotes"),
            totalInvalidVotes: t("resultsPortal.resultsAndParticipation.totalInvalidVotes"),
            explicitInvalidVotes: t("resultsPortal.resultsAndParticipation.explicitInvalidVotes"),
            implicitInvalidVotes: t("resultsPortal.resultsAndParticipation.implicitInvalidVotes"),
            blankVotes: t("resultsPortal.resultsAndParticipation.blankVotes"),
            blankVotesChart: t("resultsPortal.resultsAndParticipation.blankVotesChart"),
            weight: t("resultsPortal.resultsAndParticipation.weight"),
            options: t("resultsPortal.resultsAndParticipation.options"),
            castVotes: t("resultsPortal.resultsAndParticipation.castVotes"),
            castVotesPercent: t("resultsPortal.resultsAndParticipation.castVotesPercent"),
            winningPosition: t("resultsPortal.resultsAndParticipation.winningPosition"),
            votesForCandidates: t("resultsPortal.resultsAndParticipation.votesForCandidates"),
            invalidVotes: t("resultsPortal.resultsAndParticipation.invalidVotes"),
            nonVoters: t("resultsPortal.resultsAndParticipation.nonVoters"),
            others: t("resultsPortal.resultsAndParticipation.others"),
            candidate: t("resultsPortal.resultsAndParticipation.candidate"),
            round: t("resultsPortal.resultsAndParticipation.round"),
            winner: t("resultsPortal.resultsAndParticipation.winner"),
            eliminated: t("resultsPortal.resultsAndParticipation.eliminated"),
            empty: t("resultsPortal.resultsAndParticipation.empty"),
            previousRounds: t("resultsPortal.resultsAndParticipation.previousRounds"),
            nextRounds: t("resultsPortal.resultsAndParticipation.nextRounds"),
        }),
        [t]
    )

    return (
        <Box
            className="seq-results-contest"
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
                className="seq-results-contest__header"
                direction={{xs: "column", sm: "row"}}
                spacing={1.5}
                justifyContent="space-between"
                alignItems={{xs: "flex-start", sm: "center"}}
            >
                <Box className="seq-results-contest__intro">
                    <Typography className="seq-results-contest__title" component="h3" variant="h6">
                        {title}
                    </Typography>
                    <Typography
                        className="seq-results-contest__meta"
                        variant="body2"
                        color="text.secondary"
                    >
                        {electionName}
                        {manifestContest.positions
                            ? ` - ${t(
                                  manifestContest.positions === 1
                                      ? "resultsPortal.position"
                                      : "resultsPortal.position_plural",
                                  {count: manifestContest.positions}
                              )}`
                            : ""}
                        {areaName ? ` - ${areaName}` : ""}
                    </Typography>
                </Box>
                <Chip
                    className="seq-results-contest__status-chip"
                    label={
                        isPublished
                            ? t("resultsPortal.published")
                            : t("resultsPortal.notPublishedYet")
                    }
                    color={isPublished ? "success" : "default"}
                    variant={isPublished ? "filled" : "outlined"}
                    size="small"
                />
            </Stack>

            {!isPublished ? (
                <Typography
                    className="seq-results-contest__not-published"
                    sx={{mt: 3}}
                    color="text.secondary"
                >
                    {t("resultsPortal.notPublishedYet")}
                </Typography>
            ) : (
                <Box className="seq-results-contest__results">
                    <ResultsAndParticipation
                        summary={summary}
                        candidates={candidates}
                        chartName={chartName}
                        processResults={processResults}
                        labels={labels}
                    />
                </Box>
            )}
        </Box>
    )
}

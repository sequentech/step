// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useMemo} from "react"
import {Box, Chip, Stack, Typography} from "@mui/material"
import {ICountingAlgorithm, TallySheetVotingChannel, VotingStatusChannel} from "@sequentech/ui-core"
import {useTranslation} from "react-i18next"
import {
    CandidateResultRow,
    PreferentialProcessResults,
    ResultsAndParticipation,
    ResultsAndParticipationLabelOverrides,
    ResultsParticipationSummary,
    VotesByChannel,
} from "@sequentech/ui-essentials"
import {ResultsManifestContest, ResultsRow, ResultsSqliteDataset} from "@/types/results"
import {translatedLabel} from "@/services/resultLabels"
import {entityClassName} from "@/services/cssClassNames"
import {orderResultCandidates} from "@/services/resultsOrdering"

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

const getId = (value: unknown): string | number | undefined =>
    typeof value === "string" || typeof value === "number" ? value : undefined

const sameId = (left: unknown, right: unknown): boolean =>
    left !== null &&
    left !== undefined &&
    right !== null &&
    right !== undefined &&
    String(left) === String(right)

interface ResultAnnotations {
    extended_metrics?: {
        weight?: unknown
        votes_by_channel?: VotesByChannel
    }
    process_results?: PreferentialProcessResults
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const parseAnnotations = (annotations: unknown): ResultAnnotations | null => {
    let parsed = annotations

    if (typeof annotations === "string") {
        try {
            parsed = JSON.parse(annotations) as unknown
        } catch {
            return null
        }
    }

    return isRecord(parsed) ? (parsed as ResultAnnotations) : null
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
        return orderResultCandidates(dataset, manifestContest.contest_id, locale).map(
            (candidate) => {
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
            }
        )
    }, [dataset, resultsCandidates, manifestContest, locale])

    const summary = useMemo<ResultsParticipationSummary | null>(() => {
        if (!resultContest) return null

        return {
            id: getId(resultContest.id),
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
            blankVotes: getNumber(resultContest.total_blank_votes ?? resultContest.blank_votes),
            blankVotesPercent: getNumber(
                resultContest.total_blank_votes_percent ?? resultContest.blank_votes_percent
            ),
            explicitBlankVotes: getNumber(resultContest.explicit_blank_votes),
            explicitBlankVotesPercent: getNumber(resultContest.explicit_blank_votes_percent),
            implicitBlankVotes: getNumber(resultContest.implicit_blank_votes),
            implicitBlankVotesPercent: getNumber(resultContest.implicit_blank_votes_percent),
            weight: getNumber(
                parseAnnotations(resultContest.annotations)?.extended_metrics?.weight
            ),
            votesByChannel: parseAnnotations(resultContest.annotations)?.extended_metrics
                ?.votes_by_channel,
        }
    }, [resultContest])

    const processResults = useMemo<PreferentialProcessResults | null>(() => {
        if (contest?.counting_algorithm !== ICountingAlgorithm.INSTANT_RUNOFF) {
            return null
        }

        return parseAnnotations(resultContest?.annotations)?.process_results ?? null
    }, [contest?.counting_algorithm, resultContest?.annotations])
    const preferential = contest?.counting_algorithm === ICountingAlgorithm.INSTANT_RUNOFF

    const title =
        (typeof resultContest?.name === "string" && resultContest.name.length > 0
            ? resultContest.name
            : undefined) ??
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
    // The manifest is fetched JSON cast to `ResultsManifest`, so `positions: number` is an
    // assertion rather than a guarantee. Coerce it: i18next skips pluralisation entirely when
    // `count` is a string and falls back to the unsuffixed key, which these messages do not have.
    const positions = Number(manifestContest.positions)
    const positionsLabel =
        Number.isFinite(positions) && positions > 0
            ? t("resultsPortal.position", {count: positions})
            : null
    const chartName = areaName
        ? `${electionName} - ${title} - ${areaName}`
        : `${electionName} - ${title}`
    const isPublished = manifestContest.publication_state === "published"
    const labels = useMemo<ResultsAndParticipationLabelOverrides>(
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
            explicitBlankVotes: t("resultsPortal.resultsAndParticipation.explicitBlankVotes"),
            implicitBlankVotes: t("resultsPortal.resultsAndParticipation.implicitBlankVotes"),
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
            participationByChannel: t(
                "resultsPortal.resultsAndParticipation.participationByChannel"
            ),
            channel: t("resultsPortal.resultsAndParticipation.channel"),
            channelNames: {
                [VotingStatusChannel.Online]: t(
                    "resultsPortal.resultsAndParticipation.channelOnline"
                ),
                [VotingStatusChannel.Kiosk]: t(
                    "resultsPortal.resultsAndParticipation.channelKiosk"
                ),
                [VotingStatusChannel.EarlyVoting]: t(
                    "resultsPortal.resultsAndParticipation.channelEarlyVoting"
                ),
                [VotingStatusChannel.Telephone]: t(
                    "resultsPortal.resultsAndParticipation.channelTelephone"
                ),
                [TallySheetVotingChannel.Paper]: t(
                    "resultsPortal.resultsAndParticipation.channelPaper"
                ),
                [TallySheetVotingChannel.Postal]: t(
                    "resultsPortal.resultsAndParticipation.channelPostal"
                ),
                [TallySheetVotingChannel.InPerson]: t(
                    "resultsPortal.resultsAndParticipation.channelInPerson"
                ),
            },
        }),
        [t]
    )

    return (
        <Box
            className={[
                "seq-results-contest",
                entityClassName("election", manifestContest.election_id),
                entityClassName("contest", manifestContest.contest_id),
                entityClassName("area", manifestContest.area_id),
                `seq-results-contest--${manifestContest.publication_state}`,
                preferential
                    ? "seq-results-contest--preferential"
                    : "seq-results-contest--candidate",
            ].join(" ")}
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
                        {positionsLabel ? ` - ${positionsLabel}` : ""}
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
                        preferential={preferential}
                        labels={labels}
                    />
                </Box>
            )}
        </Box>
    )
}

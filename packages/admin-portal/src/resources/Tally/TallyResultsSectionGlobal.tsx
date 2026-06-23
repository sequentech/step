// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo} from "react"
import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Results_Contest,
    Sequent_Backend_Results_Contest_Candidate,
} from "../../gql/graphql"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Candidate_Extended} from "./types"
import {useAtomValue} from "jotai"
import {sortCandidates} from "@/utils/candidateSort"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {ICountingAlgorithm} from "@sequentech/ui-core"
import {parseProcessResults} from "./utils"
import {RunoffStatus} from "./types"
import {LoadingResults} from "./TallyElectionsResults"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {useDefaultElectionLang} from "@/hooks/useDefaultElectionLang"
import {
    CandidateResultRow,
    PreferentialProcessResults,
    ResultsAndParticipation,
    ResultsAndParticipationLabels,
    ResultsParticipationSummary,
} from "@sequentech/ui-essentials"

interface TallyResultsGlobalCandidatesProps {
    contestId: string
    electionId: string
    electionEventId: string
    tenantId: string
    resultsEventId: string | null
    counting_algorithm: ICountingAlgorithm
}

export const TallyResultsSectionGlobal: React.FC<TallyResultsGlobalCandidatesProps> = (props) => {
    const {contestId, electionId, electionEventId, tenantId, resultsEventId, counting_algorithm} =
        props
    const {t, i18n} = useTranslation()
    const tallyData = useAtomValue(tallyQueryData)
    const aliasRenderer = useAliasRenderer()
    const defaultElectionLang = useDefaultElectionLang(electionId, electionEventId)

    const candidates: Array<Sequent_Backend_Candidate> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_candidate?.filter(
                (candidate) => contestId === candidate.contest_id
            ),
        [tallyData?.sequent_backend_candidate, contestId]
    )

    const general: Array<Sequent_Backend_Results_Contest> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_results_contest?.filter(
                (resultsContest) =>
                    contestId === resultsContest.contest_id &&
                    electionId === resultsContest.election_id
            ),
        [tallyData?.sequent_backend_results_contest, contestId, electionId]
    )

    const results: Array<Sequent_Backend_Results_Contest_Candidate> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_results_contest_candidate?.filter(
                (resultsContestCandidate) =>
                    contestId === resultsContestCandidate.contest_id &&
                    electionId === resultsContestCandidate.election_id
            ),
        [tallyData?.sequent_backend_results_contest_candidate, contestId, electionId]
    )

    const resultsData = useMemo<Array<Sequent_Backend_Candidate_Extended>>(() => {
        if (!results || !candidates) return []

        return candidates.map((candidate, index): Sequent_Backend_Candidate_Extended => {
            const candidateResult = results.find((r) => r.candidate_id === candidate.id)
            const candidateName = aliasRenderer(candidate.presentation, defaultElectionLang)

            return {
                ...candidate,
                rowId: index,
                id: candidate.id || "",
                name: candidateName,
                status: "",
                cast_votes: candidateResult?.cast_votes,
                cast_votes_percent: candidateResult?.cast_votes_percent,
                winning_position: candidateResult?.winning_position,
            }
        })
    }, [results, candidates, aliasRenderer, defaultElectionLang])

    const orderedResultsData = useMemo(() => {
        return [...resultsData].sort(sortCandidates)
    }, [resultsData])

    const contestName = useMemo(() => {
        if (!contestId || !tallyData) return undefined

        const contest = tallyData?.sequent_backend_contest?.find(
            (contest) => contest.id === contestId
        )
        if (!contest?.presentation) return undefined

        return aliasRenderer(contest.presentation)
    }, [contestId, tallyData, i18n.language, aliasRenderer])

    const electionName: string | undefined = useMemo(() => {
        const election = tallyData?.sequent_backend_election?.find(
            (election) => election.id === electionId
        )
        return election?.presentation ? aliasRenderer(election.presentation) : undefined
    }, [tallyData?.sequent_backend_election, electionId, aliasRenderer])

    const processResults = useMemo(
        () =>
            parseProcessResults(
                general?.[0]?.annotations,
                counting_algorithm
            ) as RunoffStatus | null,
        [general?.[0]?.annotations, counting_algorithm]
    )

    const summary = useMemo<ResultsParticipationSummary | null>(() => {
        const result = general?.[0]
        if (!result) return null

        return {
            id: result.id,
            eligibleCensus: result.elegible_census,
            totalAuditableVotes: result.total_auditable_votes,
            totalAuditableVotesPercent: result.total_auditable_votes_percent,
            totalVotes: result.total_votes,
            totalVotesPercent: result.total_votes_percent,
            totalValidVotes: result.total_valid_votes,
            totalValidVotesPercent: result.total_valid_votes_percent,
            totalInvalidVotes: result.total_invalid_votes,
            totalInvalidVotesPercent: result.total_invalid_votes_percent,
            explicitInvalidVotes: result.explicit_invalid_votes,
            explicitInvalidVotesPercent: result.explicit_invalid_votes_percent,
            implicitInvalidVotes: result.implicit_invalid_votes,
            implicitInvalidVotesPercent: result.implicit_invalid_votes_percent,
            blankVotes: result.blank_votes,
            blankVotesPercent: result.blank_votes_percent,
        }
    }, [general])

    const resultRows = useMemo<CandidateResultRow[]>(
        () =>
            orderedResultsData.map((candidate) => ({
                id: candidate.id,
                name: candidate.name ?? "-",
                castVotes: candidate.cast_votes,
                castVotesPercent: candidate.cast_votes_percent,
                winningPosition: candidate.winning_position,
            })),
        [orderedResultsData]
    )

    const labels = useMemo<Partial<ResultsAndParticipationLabels>>(
        () => ({
            participationSummary: t("tally.table.global"),
            candidateResults: t("tally.table.candidates"),
            total: t("tally.table.total"),
            turnout: t("tally.table.turnout"),
            eligibleCensus: t("tally.table.elegible_census"),
            totalAuditableVotes: t("tally.table.total_auditable_votes"),
            totalVotesCounted: t("tally.table.total_votes_counted"),
            totalValidVotes: t("tally.table.total_valid_votes"),
            totalInvalidVotes: t("tally.table.total_invalid_votes"),
            explicitInvalidVotes: t("tally.table.explicit_invalid_votes"),
            implicitInvalidVotes: t("tally.table.implicit_invalid_votes"),
            blankVotes: t("tally.table.blank_votes"),
            blankVotesChart: t("tally.chart.blankVotes"),
            weight: t("tally.table.weight"),
            options: t("tally.table.options"),
            castVotes: t("tally.table.cast_votes"),
            castVotesPercent: t("tally.table.cast_votes_percent"),
            winningPosition: t("tally.table.winning_position"),
            votesForCandidates: t("tally.chart.votesForCandidates"),
            invalidVotes: t("tally.chart.invalidVotes"),
            nonVoters: t("tally.chart.nonVoters"),
            candidate: t("tally.table.preferential.candidate"),
            round: t("tally.table.preferential.round"),
            winner: t("tally.table.preferential.winner"),
            eliminated: t("tally.table.preferential.eliminated"),
            empty: t("common.label.noResult"),
        }),
        [t]
    )

    const getChartName = () => {
        if (electionName && contestName) {
            return `${electionName} - ${contestName} - ` + t("tally.common.global")
        } else {
            return "-"
        }
    }

    const isTallyDataMatchCurrentResults = useMemo(() => {
        return (
            resultsEventId &&
            !!tallyData?.sequent_backend_results_event.find((event) => event.id === resultsEventId)
        )
    }, [tallyData?.sequent_backend_results_event, resultsEventId])

    return (
        <>
            {!isTallyDataMatchCurrentResults ? (
                <LoadingResults />
            ) : (
                <ResultsAndParticipation
                    summary={summary}
                    candidates={resultRows}
                    chartName={getChartName()}
                    labels={labels}
                    processResults={
                        counting_algorithm === ICountingAlgorithm.INSTANT_RUNOFF
                            ? (processResults as PreferentialProcessResults | null)
                            : null
                    }
                />
            )}
        </>
    )
}

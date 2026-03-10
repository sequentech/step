// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Results_Contest,
    Sequent_Backend_Results_Contest_Candidate,
} from "../../gql/graphql"
import {useTranslation} from "react-i18next"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {Sequent_Backend_Candidate_Extended} from "./types"
import {useAtomValue} from "jotai"
import {sortCandidates} from "@/utils/candidateSort"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {TallyResultsSummary} from "./TallyResultsSummary"
import {TallyResultsCandidatesPlurality} from "./TallyResultsCandidatesPlurality"
import {TallyResultsCandidatesIRV} from "./TallyResultsCandidatesIRV"
import {ICountingAlgorithm} from "@sequentech/ui-core"
import {winningPositionComparator, parseProcessResults} from "./utils"
import {RunoffStatus} from "./types"
import {LoadingResults} from "./TallyElectionsResults"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"

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
    const {globalSettings} = useContext(SettingsContext)
    const tallyData = useAtomValue(tallyQueryData)
    const aliasRenderer = useAliasRenderer()

    const [resultsData, setResultsData] = useState<Array<Sequent_Backend_Candidate_Extended>>([])
    const orderedResultsData = useMemo(() => {
        return resultsData.sort(sortCandidates)
    }, [resultsData])

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

    const contestName = useMemo(() => {
        if (!contestId || !tallyData) return undefined

        const contest = tallyData?.sequent_backend_contest?.find(
            (contest) => contest.id === contestId
        )
        if (!contest?.presentation) return undefined

        return aliasRenderer(contest.presentation)
    }, [contestId, tallyData, i18n.language])

    const electionName: string | undefined = useMemo(() => {
        const election = tallyData?.sequent_backend_election?.find(
            (election) => election.id === electionId
        )
        return election?.presentation ? aliasRenderer(election.presentation) : undefined
    }, [tallyData?.sequent_backend_election, electionId])

    const processResults = useMemo(
        () =>
            parseProcessResults(
                general?.[0]?.annotations,
                counting_algorithm
            ) as RunoffStatus | null,
        [general?.[0]?.annotations, counting_algorithm]
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

    useEffect(() => {
        if (results && candidates) {
            const temp: Array<Sequent_Backend_Candidate_Extended> | undefined = candidates?.map(
                (candidate, index): Sequent_Backend_Candidate_Extended => {
                    let candidateResult = results.find((r) => r.candidate_id === candidate.id)

                    let candidateName = aliasRenderer(candidate.presentation)
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
                }
            )

            setResultsData(temp)
        }
    }, [results, candidates, i18n.language])

    return (
        <>
            {!isTallyDataMatchCurrentResults ? (
                <LoadingResults />
            ) : (
                <>
                    <TallyResultsSummary general={general} chartName={getChartName()} />
                    {counting_algorithm === ICountingAlgorithm.PLURALITY_AT_LARGE && (
                        <TallyResultsCandidatesPlurality
                            resultsData={resultsData}
                            orderedResultsData={orderedResultsData}
                            chartName={getChartName()}
                        />
                    )}
                    {counting_algorithm === ICountingAlgorithm.INSTANT_RUNOFF && processResults && (
                        <TallyResultsCandidatesIRV processResults={processResults} />
                    )}
                </>
            )}
        </>
    )
}

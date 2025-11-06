// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
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
import {ICountingAlgorithm} from "../Contest/constants"
import {winningPositionComparator, parseProcessResults} from "./utils"
import {RunoffStatus} from "./types"

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
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const tallyData = useAtomValue(tallyQueryData)

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

    const electionName: string | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_election?.find((election) => election.id === electionId)
                ?.name,
        [tallyData?.sequent_backend_election, electionId]
    )

    const processResults = useMemo(
        () =>
            parseProcessResults(
                general?.[0]?.annotations,
                counting_algorithm
            ) as RunoffStatus | null,
        [general?.[0]?.annotations, counting_algorithm]
    )

    const getChartName = (contestName: string | undefined) => {
        if (electionName && contestName) {
            return `${electionName} - ${contestName} - ` + t("tally.common.global")
        } else {
            return "-"
        }
    }

    useEffect(() => {
        if (results && candidates) {
            const temp: Array<Sequent_Backend_Candidate_Extended> | undefined = candidates?.map(
                (candidate, index): Sequent_Backend_Candidate_Extended => {
                    let candidateResult = results.find((r) => r.candidate_id === candidate.id)

                    return {
                        ...candidate,
                        rowId: index,
                        id: candidate.id || "",
                        name: candidate.name,
                        status: "",
                        cast_votes: candidateResult?.cast_votes,
                        cast_votes_percent: candidateResult?.cast_votes_percent,
                        winning_position: candidateResult?.winning_position,
                    }
                }
            )

            setResultsData(temp)
        }
    }, [results, candidates])

    return (
        <>
            <TallyResultsSummary
                general={general}
                chartName={getChartName(general?.[0].name ?? undefined)}
            />
            {counting_algorithm === ICountingAlgorithm.PLURALITY_AT_LARGE && (
                <TallyResultsCandidatesPlurality
                    resultsData={resultsData}
                    orderedResultsData={orderedResultsData}
                    chartName={getChartName(general?.[0].name ?? undefined)}
                />
            )}
            {counting_algorithm === ICountingAlgorithm.INSTANT_RUNOFF && processResults && (
                <TallyResultsCandidatesIRV processResults={processResults} />
            )}
        </>
    )
}

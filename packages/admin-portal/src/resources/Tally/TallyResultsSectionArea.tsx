// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {useRecordContext} from "react-admin"
import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Results_Area_Contest,
    Sequent_Backend_Results_Area_Contest_Candidate,
    Sequent_Backend_Election_Event,
} from "../../gql/graphql"
import {useTranslation} from "react-i18next"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {Sequent_Backend_Candidate_Extended, ParsedAnnotations, RunoffStatus} from "./types"
import {useAtomValue} from "jotai"
import {sortCandidates} from "@/utils/candidateSort"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {EElectionEventWeightedVotingPolicy} from "@sequentech/ui-core"
import {TallyResultsSummary} from "./TallyResultsSummary"
import {TallyResultsCandidatesPlurality} from "./TallyResultsCandidatesPlurality"
import {TallyResultsCandidatesIRV} from "./TallyResultsCandidatesIRV"
import {ICountingAlgorithm} from "../Contest/constants"
import {winningPositionComparator, parseProcessResults} from "./utils"

interface TallyResultsCandidatesProps {
    areaId: string | null | undefined
    contestId: string
    electionId: string
    electionEventId: string
    tenantId: string
    resultsEventId: string | null
    counting_algorithm: ICountingAlgorithm
}

export const TallyResultsSectionArea: React.FC<TallyResultsCandidatesProps> = (props) => {
    const {areaId, contestId, electionId, electionEventId, tenantId, resultsEventId, counting_algorithm} = props
    const [resultsData, setResultsData] = useState<Array<Sequent_Backend_Candidate>>([])
    const orderedResultsData = useMemo(() => {
        return (resultsData as Sequent_Backend_Candidate_Extended[]).sort(sortCandidates)
    }, [resultsData])
    const {t} = useTranslation()
    const {globalSettings} = useContext(SettingsContext)
    const tallyData = useAtomValue(tallyQueryData)

    const candidates: Array<Sequent_Backend_Candidate> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_candidate
                ?.filter((candidate) => contestId === candidate.contest_id)
                ?.map((candidate): Sequent_Backend_Candidate => candidate),
        [tallyData?.sequent_backend_candidate, contestId]
    )

    const general: Array<Sequent_Backend_Results_Area_Contest> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_results_area_contest?.filter(
                (areaContest) =>
                    contestId === areaContest.contest_id &&
                    electionId === areaContest.election_id &&
                    areaId === areaContest.area_id
            ),
        [tallyData?.sequent_backend_results_area_contest, contestId, electionId, areaId]
    )

    const results: Array<Sequent_Backend_Results_Area_Contest_Candidate> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_results_area_contest_candidate?.filter(
                (areaContestCandidate) =>
                    contestId === areaContestCandidate.contest_id &&
                    electionId === areaContestCandidate.election_id &&
                    areaId === areaContestCandidate.area_id
            ),
        [tallyData?.sequent_backend_results_area_contest_candidate, contestId, electionId]
    )

    const weight = useMemo((): number | null => {
        try {
            const parsedAnnotations: ParsedAnnotations | null = general?.[0]?.annotations
                ? (JSON.parse(general[0].annotations as string) as ParsedAnnotations)
                : null
            return parsedAnnotations?.extended_metrics?.weight ?? null
        } catch {
            return null
        }
    }, [general?.[0]])

    const processResults = useMemo(
        () => parseProcessResults(general?.[0]?.annotations, counting_algorithm) as RunoffStatus | null,
        [general?.[0]?.annotations, counting_algorithm]
    )

    const eventRecord = useRecordContext<Sequent_Backend_Election_Event>()
    const weightedVotingForAreas = useMemo((): boolean => {
        return (
            eventRecord?.presentation?.weighted_voting_policy ===
            EElectionEventWeightedVotingPolicy.AREAS_WEIGHTED_VOTING
        )
    }, [eventRecord])

    const electionName: string | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_election?.find((election) => election.id === electionId)
                ?.name,
        [tallyData?.sequent_backend_election, electionId]
    )

    const contestName: string | undefined | null = useMemo(
        () => tallyData?.sequent_backend_contest?.find((contest) => contest.id === contestId)?.name,
        [tallyData?.sequent_backend_contest, contestId]
    )

    const areaName: string | undefined | null = useMemo(
        () => tallyData?.sequent_backend_area?.find((area) => area.id === areaId)?.name,
        [tallyData?.sequent_backend_area, areaId]
    )

    const getChartName = () => {
        if (electionName && contestName && areaName) {
            return `${electionName} - ${contestName} - ${areaName}`
        } else {
            return "-"
        }
    }

    useEffect(() => {
        if (results && candidates) {
            const temp: Array<Sequent_Backend_Candidate_Extended> | undefined = candidates?.map(
                (candidate, index) => {
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
                chartName={getChartName()}
                showWeight={weightedVotingForAreas}
                weight={weight}
            />
            {counting_algorithm === ICountingAlgorithm.PLURALITY_AT_LARGE && (
                <TallyResultsCandidatesPlurality
                    resultsData={resultsData as Sequent_Backend_Candidate_Extended[]}
                    orderedResultsData={orderedResultsData}
                    chartName={getChartName()}
                />
            )}
            {counting_algorithm === ICountingAlgorithm.INSTANT_RUNOFF && processResults && (
                <TallyResultsCandidatesIRV processResults={processResults} />
            )}
        </>
    )
}

// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {
    Sequent_Backend_Candidate,
    Sequent_Backend_Results_Contest,
    Sequent_Backend_Results_Contest_Candidate,
} from "../../gql/graphql"
import {DataGrid, GridColDef, GridRenderCellParams, GridComparatorFn} from "@mui/x-data-grid"
import {useTranslation} from "react-i18next"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {Sequent_Backend_Candidate_Extended} from "./types"
import {formatPercentOne, isNumber} from "@sequentech/ui-core"
import {useAtomValue} from "jotai"
import {sortCandidates} from "@/utils/candidateSort"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {TallyResultsSummary} from "./TallyResultsSummary"
import {TallyResultsCandidatesPlurality} from "./TallyResultsCandidatesPlurality"
import {ICountingAlgorithm} from "../Contest/constants"
interface TallyResultsGlobalCandidatesProps {
    contestId: string
    electionId: string
    electionEventId: string
    tenantId: string
    resultsEventId: string | null
    counting_algorithm: ICountingAlgorithm
}

// Define the comparator function
const winningPositionComparator: GridComparatorFn<string> = (v1, v2) => {
    const maxInt = Number.MAX_SAFE_INTEGER

    // Convert stringified numbers to integers, non-numeric strings to maxInt
    const pos1 = isNaN(parseInt(v1)) ? maxInt : parseInt(v1)
    const pos2 = isNaN(parseInt(v2)) ? maxInt : parseInt(v2)

    return pos1 - pos2
}

export const TallyResultsSectionGlobal: React.FC<TallyResultsGlobalCandidatesProps> = (
    props
) => {
    const {contestId, electionId, electionEventId, tenantId, resultsEventId, counting_algorithm} = props
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

    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: t("tally.table.options"),
            flex: 1,
            editable: false,
            align: "left",
        },
        {
            field: "cast_votes",
            headerName: t("tally.table.cast_votes"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) => props["value"] ?? "-",
            align: "right",
            headerAlign: "right",
        },
        {
            field: "cast_votes_percent",
            headerName: t("tally.table.cast_votes_percent"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, string>) =>
                isNumber(props["value"]) ? formatPercentOne(props["value"]) : "-",
            align: "right",
            headerAlign: "right",
        },
        {
            field: "winning_position",
            headerName: t("tally.table.winning_position"),
            flex: 1,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, number>) => props["value"] ?? "-",
            sortComparator: winningPositionComparator,
            align: "right",
            headerAlign: "right",
        },
    ]

    return (
        <>
            <TallyResultsSummary
                general={general}
                chartName={getChartName(general?.[0].name ?? undefined)}
            />
            <TallyResultsCandidatesPlurality
                resultsData={resultsData}
                orderedResultsData={orderedResultsData}
                columns={columns}
                chartName={getChartName(general?.[0].name ?? undefined)}
            />
        </>
    )
}

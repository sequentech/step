// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {memo, useCallback, useMemo} from "react"
import {Identifier, RaRecord, useRecordContext} from "react-admin"
import {Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useAtomValue} from "jotai"
import {
    IContest,
    ICountingAlgorithm,
    IElectionEventPresentation,
    IElectionPresentation,
    parseEntityPresentation,
    sortByPresentationOrder,
    sortContestList,
} from "@sequentech/ui-core"
import {
    ResultsSelectorAreaOption,
    ResultsSelectorOption,
    ResultsSelectorSelection,
    ResultsSelectorTabs,
} from "@sequentech/ui-essentials"
import {
    Sequent_Backend_Area_Contest,
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tally_Session,
    GetTallyDataQuery,
} from "../../gql/graphql"
import {ExportElectionMenu, IResultDocumentsData} from "@/components/tally/ExportElectionMenu"
import {IResultDocuments} from "@/types/results"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {useKeysPermissions} from "../ElectionEvent/useKeysPermissions"
import {getDefaultElectionLang} from "@/hooks/useDefaultElectionLang"
import {TallyResultsSectionGlobal} from "./TallyResultsSectionGlobal"
import {TallyResultsSectionArea} from "./TallyResultsSectionArea"
import {convertContestsArray} from "./utils"
import {LoadingResults} from "./TallyElectionsResults"

interface TallyResultsProps {
    tally: Sequent_Backend_Tally_Session | undefined
    resultsEventId: string | null
    loading?: boolean
    onCreateTransmissionPackage: (v: {area_id: string; election_id: string}) => void
}

type ElectionOption = ResultsSelectorOption<Sequent_Backend_Election>
type ContestOption = ResultsSelectorOption<IContest>
type AreaOption = ResultsSelectorAreaOption<Sequent_Backend_Area_Contest | null>
type TallyContestRow = GetTallyDataQuery["sequent_backend_contest"][number]
type TallyResultsSelection = ResultsSelectorSelection<
    Sequent_Backend_Election,
    IContest,
    Sequent_Backend_Area_Contest | null
>

const parseDocuments = (rawDocuments: unknown): IResultDocuments | null => {
    if (!rawDocuments) return null

    try {
        return typeof rawDocuments === "string"
            ? (JSON.parse(rawDocuments) as IResultDocuments)
            : (rawDocuments as IResultDocuments)
    } catch (error) {
        console.error("Failed to parse documents JSON string:", error)
        return null
    }
}

const withoutNestedContestData = (contest: TallyContestRow): Sequent_Backend_Contest => ({
    ...contest,
    candidates: [],
    candidates_aggregate: {nodes: []},
})

const TallyResultsElectionsTabs: React.MemoExoticComponent<React.FC<TallyResultsProps>> = memo(
    (props: TallyResultsProps): React.JSX.Element => {
        const {tally, resultsEventId, onCreateTransmissionPackage, loading} = props

        const {t, i18n} = useTranslation()
        const tallyData = useAtomValue(tallyQueryData)
        const electionEventRecord = useRecordContext<Sequent_Backend_Election_Event>()
        const aliasRenderer = useAliasRenderer()
        const {canExportCeremony} = useKeysPermissions()

        const areas: Array<RaRecord<Identifier>> | undefined = useMemo(
            () => tallyData?.sequent_backend_area?.map((area): RaRecord<Identifier> => area),
            [tallyData?.sequent_backend_area]
        )

        const unorderedElections = useMemo<Sequent_Backend_Election[]>(() => {
            const electionById = new Map(
                tallyData?.sequent_backend_election?.map((election) => [election.id, election]) ??
                    []
            )

            return (tally?.election_ids ?? [])
                .map((electionId) => electionById.get(electionId))
                .filter((election): election is Sequent_Backend_Election => !!election)
                .map(
                    (election): Sequent_Backend_Election => ({
                        ...election,
                        contests: [],
                        contests_aggregate: {nodes: []},
                    })
                )
        }, [tally?.election_ids, tallyData?.sequent_backend_election])

        const defaultLangByElectionId = useMemo(() => {
            const map = new Map<string, string | undefined>()
            unorderedElections.forEach((election) => {
                map.set(
                    election.id,
                    getDefaultElectionLang(tallyData, election.id, election.election_event_id)
                )
            })
            return map
        }, [unorderedElections, tallyData?.sequent_backend_election])

        const getElectionAlias = useCallback(
            (election: Sequent_Backend_Election) =>
                aliasRenderer(election.presentation, defaultLangByElectionId.get(election.id)),
            [aliasRenderer, defaultLangByElectionId]
        )

        const getContestAlias = useCallback(
            (contest: unknown, electionId?: string | null) =>
                aliasRenderer(
                    contest,
                    electionId ? defaultLangByElectionId.get(electionId) : undefined
                ),
            [aliasRenderer, defaultLangByElectionId]
        )

        const electionsOrder = parseEntityPresentation<IElectionEventPresentation>(
            electionEventRecord?.presentation
        )?.elections_order

        const elections = useMemo(
            () =>
                sortByPresentationOrder(unorderedElections, electionsOrder, {
                    getLabel: getElectionAlias,
                    getPresentation: (election) => election.presentation,
                }),
            [electionsOrder, getElectionAlias, unorderedElections]
        )

        const electionOptions = useMemo<ElectionOption[]>(
            () =>
                elections.map((election) => ({
                    id: election.id,
                    label: getElectionAlias(election),
                    data: election,
                })),
            [elections, getElectionAlias]
        )

        const contestsByElectionId = useMemo(() => {
            const map = new Map<string, IContest[]>()
            const contests = tallyData?.sequent_backend_contest ?? []

            elections.forEach((election) => {
                const backendContests = contests
                    .map(withoutNestedContestData)
                    .filter((contest) => contest.election_id === election.id)
                const contestOrder = parseEntityPresentation<IElectionPresentation>(
                    election.presentation
                )?.contests_order
                const convertedContests = convertContestsArray(backendContests)
                map.set(election.id, sortContestList(convertedContests, contestOrder))
            })

            return map
        }, [elections, tallyData?.sequent_backend_contest])

        const getContestOptions = useCallback(
            (selection: Pick<TallyResultsSelection, "election">): ContestOption[] => {
                const contests = selection.election
                    ? (contestsByElectionId.get(selection.election.id) ?? [])
                    : []

                return contests.map((contest) => ({
                    id: contest.id,
                    label: aliasRenderer(contest),
                    data: contest,
                }))
            },
            [aliasRenderer, contestsByElectionId, i18n.language]
        )

        const getAreaOptions = useCallback(
            (selection: Pick<TallyResultsSelection, "contest">): AreaOption[] => {
                const contestId = selection.contest?.id
                const contestAreas =
                    tallyData?.sequent_backend_area_contest?.filter(
                        (areaContest) => contestId === areaContest.contest_id
                    ) ?? []

                return [
                    {
                        id: null,
                        label: String(t("tally.common.global")),
                        data: null,
                    },
                    ...contestAreas.map((areaContest) => ({
                        id: areaContest.area_id,
                        label: aliasRenderer(
                            areas?.find((item) => item.id === areaContest.area_id)
                        ),
                        data: areaContest,
                    })),
                ]
            },
            [aliasRenderer, areas, tallyData?.sequent_backend_area_contest, t]
        )

        const getElectionDocuments = useCallback(
            (selection: TallyResultsSelection): IResultDocumentsData[] | null => {
                const electionId = selection.electionId
                if (!resultsEventId || !electionId) return null

                const resultsElection = tallyData?.sequent_backend_results_election?.find(
                    (election) =>
                        election.results_event_id === resultsEventId &&
                        election.election_id === electionId
                )
                const electionDocuments = parseDocuments(resultsElection?.documents)
                const documentsList: IResultDocumentsData[] = []

                if (electionDocuments) {
                    documentsList.push({
                        documents: electionDocuments,
                        name: selection.election?.data
                            ? getElectionAlias(selection.election.data)
                            : "election",
                        class_type: "election",
                    })
                }

                tallyData?.sequent_backend_results_election_area
                    ?.filter(
                        (area) =>
                            area.results_event_id === resultsEventId &&
                            area.election_id === electionId
                    )
                    .forEach((area) => {
                        const areaDocuments = parseDocuments(area.documents)
                        if (!areaDocuments) return

                        documentsList.push({
                            documents: areaDocuments,
                            name: area.name ?? "area",
                            class_type: "election",
                            class_subtype: "election-area",
                        })
                    })

                return documentsList.length ? documentsList : null
            },
            [
                getElectionAlias,
                resultsEventId,
                tallyData?.sequent_backend_results_election,
                tallyData?.sequent_backend_results_election_area,
            ]
        )

        const getContestDocuments = useCallback(
            (selection: TallyResultsSelection): IResultDocumentsData | null => {
                const contestId = selection.contestId
                const electionId = selection.electionId
                if (!contestId || !electionId) return null

                const resultsContest = tallyData?.sequent_backend_results_contest?.find(
                    (contest) =>
                        contest.contest_id === contestId &&
                        contest.election_id === electionId &&
                        (!resultsEventId || contest.results_event_id === resultsEventId)
                )
                const documents = parseDocuments(resultsContest?.documents)

                return documents
                    ? {
                          documents,
                          name: selection.contest?.data
                              ? aliasRenderer(selection.contest.data)
                              : "contest",
                          class_type: "contest",
                      }
                    : null
            },
            [aliasRenderer, resultsEventId, tallyData?.sequent_backend_results_contest]
        )

        const getAreaDocuments = useCallback(
            (selection: TallyResultsSelection): IResultDocumentsData | null => {
                const contestId = selection.contestId
                const electionId = selection.electionId
                const areaId = selection.areaId
                if (!contestId || !electionId || !areaId) return null

                const resultsContest = tallyData?.sequent_backend_results_area_contest?.find(
                    (areaContest) =>
                        areaContest.contest_id === contestId &&
                        areaContest.election_id === electionId &&
                        areaContest.area_id === areaId &&
                        (!resultsEventId || areaContest.results_event_id === resultsEventId)
                )
                const documents = parseDocuments(resultsContest?.documents)

                return documents
                    ? {
                          documents,
                          name: selection.contest?.data
                              ? aliasRenderer(selection.contest.data)
                              : "contest",
                          class_type: "contest-area",
                      }
                    : null
            },
            [aliasRenderer, resultsEventId, tallyData?.sequent_backend_results_area_contest]
        )

        const renderElectionActions = useCallback(
            (selection: TallyResultsSelection) => {
                const documentsList = getElectionDocuments(selection)
                if (!documentsList || !canExportCeremony || !tally?.id) return null

                return (
                    <ExportElectionMenu
                        documentsList={documentsList}
                        electionEventId={selection.election?.data?.election_event_id}
                        itemName={
                            selection.election?.data
                                ? getElectionAlias(selection.election.data)
                                : "election"
                        }
                        tallyType={tally.tally_type}
                        electionId={selection.electionId}
                        onCreateTransmissionPackage={onCreateTransmissionPackage}
                        miruExportloading={loading}
                        tallySessionId={tally.id}
                    />
                )
            },
            [
                canExportCeremony,
                getElectionAlias,
                getElectionDocuments,
                loading,
                onCreateTransmissionPackage,
                tally?.id,
                tally?.tally_type,
            ]
        )

        const renderContestActions = useCallback(
            (selection: TallyResultsSelection) => {
                const documents = getContestDocuments(selection)
                if (
                    !documents ||
                    !selection.election?.data?.election_event_id ||
                    !canExportCeremony ||
                    !tally?.id
                ) {
                    return null
                }

                return (
                    <ExportElectionMenu
                        documentsList={[documents]}
                        electionEventId={selection.election.data.election_event_id}
                        itemName={
                            selection.contest?.data
                                ? aliasRenderer(selection.contest.data)
                                : "contest"
                        }
                        tallySessionId={tally.id}
                    />
                )
            },
            [aliasRenderer, canExportCeremony, getContestDocuments, tally?.id]
        )

        const renderAreaActions = useCallback(
            (selection: TallyResultsSelection) => {
                const documents = getAreaDocuments(selection)
                if (
                    !documents ||
                    !selection.election?.data?.election_event_id ||
                    !canExportCeremony ||
                    !tally?.id
                ) {
                    return null
                }

                return (
                    <ExportElectionMenu
                        documentsList={[documents]}
                        electionEventId={selection.election.data.election_event_id}
                        itemName={
                            selection.contest?.data
                                ? aliasRenderer(selection.contest.data)
                                : "contest"
                        }
                        tallySessionId={tally.id}
                    />
                )
            },
            [aliasRenderer, canExportCeremony, getAreaDocuments, tally?.id]
        )

        const renderResult = useCallback(
            (selection: TallyResultsSelection) => {
                const contest = selection.contest?.data
                if (!contest) {
                    return (
                        <Typography color="text.secondary" sx={{mt: 3}}>
                            {t("common.label.noResult")}
                        </Typography>
                    )
                }

                if (!selection.areaId) {
                    return (
                        <TallyResultsSectionGlobal
                            electionEventId={contest.election_event_id}
                            tenantId={contest.tenant_id}
                            electionId={contest.election_id}
                            contestId={contest.id}
                            resultsEventId={resultsEventId}
                            counting_algorithm={contest.counting_algorithm as ICountingAlgorithm}
                        />
                    )
                }

                return (
                    <TallyResultsSectionArea
                        electionEventId={contest.election_event_id}
                        tenantId={contest.tenant_id}
                        electionId={contest.election_id}
                        contestId={contest.id}
                        areaId={selection.areaId}
                        resultsEventId={resultsEventId}
                        counting_algorithm={contest.counting_algorithm as ICountingAlgorithm}
                    />
                )
            },
            [resultsEventId, t]
        )

        if (!tallyData) {
            return <LoadingResults />
        }

        return (
            <ResultsSelectorTabs
                className="seq-admin-tally-results-selector"
                labels={{
                    elections: t("electionEventScreen.stats.elections"),
                    contests: t("electionEventScreen.stats.contests"),
                    areas: t("electionEventScreen.stats.areas"),
                    empty: t("common.label.noResult"),
                }}
                elections={electionOptions}
                getContests={getContestOptions}
                getAreas={getAreaOptions}
                renderElectionActions={renderElectionActions}
                renderContestActions={renderContestActions}
                renderAreaActions={renderAreaActions}
                renderResult={renderResult}
            />
        )
    }
)

TallyResultsElectionsTabs.displayName = "TallyResults"

export const TallyResults = TallyResultsElectionsTabs

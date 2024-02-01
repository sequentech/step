// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {useGetList, useGetOne} from "react-admin"

import {
    Sequent_Backend_Area_Contest,
    Sequent_Backend_Contest,
    Sequent_Backend_Results_Area_Contest,
    Sequent_Backend_Results_Area_Contest_Candidate,
    Sequent_Backend_Results_Contest,
    Sequent_Backend_Results_Contest_Candidate,
} from "../../gql/graphql"
import {Box, Tabs, Tab, Typography} from "@mui/material"
import * as reactI18next from "react-i18next"
import {TallyResultsGlobalCandidates} from "./TallyResultsGlobalCandidates"
import {TallyResultsCandidates} from "./TallyResultsCandidates"
import {ExportElectionMenu} from "@/components/tally/ExportElectionMenu"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {IResultDocuments} from "@/types/results"
import {Sequent_Backend_Candidate_Extended} from "./types"
import {useAtom} from "jotai"
import tallyCandidates, {
    tallyAreaCandidates,
    tallyAreaData,
    tallyAreasContest,
    tallyCandidatesList,
    tallyGeneralData,
    tallyResultsEventId,
    tallySelectedTab,
} from "@/atoms/tally-candidates"

interface TallyResultsContestAreasProps {
    contestId: string | null
    electionId: string | null
    electionEventId: string | null
    tenantId: string | null
}

export const TallyResultsContestAreas: React.FC<TallyResultsContestAreasProps> = (props) => {
    const {contestId, electionId, electionEventId, tenantId} = props
    const {t} = reactI18next.useTranslation()
    const {globalSettings} = useContext(SettingsContext)

    const [selectedArea, setSelectedArea] = useState<string | null>()

    const [_, setResultsData] = useAtom(tallyCandidates)
    const [__, setAreaResultsData] = useAtom(tallyAreaCandidates)
    const [resultsEventId] = useAtom(tallyResultsEventId)
    const [candidatesList] = useAtom(tallyCandidatesList)
    const [contestAreas] = useAtom(tallyAreasContest)
    const [value, setValue] = useAtom(tallySelectedTab)

    const {data: resultsContests} = useGetList<Sequent_Backend_Results_Area_Contest>(
        "sequent_backend_results_area_contest",
        {
            pagination: {page: 1, perPage: 1},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventId,
                results_event_id: resultsEventId,
                election_id: electionId,
                contest_id: contestId,
                area_id: selectedArea,
            },
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const [____, setAreaData] = useAtom(tallyAreaData)
    useMemo(() => {
        setAreaData(resultsContests?.[0] || null)
    }, [resultsContests])

    const {data: contest} = useGetOne<Sequent_Backend_Contest>(
        "sequent_backend_contest",
        {
            id: contestId,
            meta: {tenant_id: tenantId},
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: general} = useGetList<Sequent_Backend_Results_Contest>(
        "sequent_backend_results_contest",
        {
            pagination: {page: 1, perPage: 1},
            filter: {
                contest_id: contestId,
                tenant_id: tenantId,
                election_event_id: electionEventId,
                election_id: electionId,
                results_event_id: resultsEventId,
            },
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const [___, setGeneralData] = useAtom(tallyGeneralData)
    useMemo(() => {
        setGeneralData(general?.[0] || null)
    }, [general])

    const {data: results} = useGetList<Sequent_Backend_Results_Contest_Candidate>(
        "sequent_backend_results_contest_candidate",
        {
            pagination: {page: 1, perPage: 9999},
            filter: {
                contest_id: contestId,
                tenant_id: tenantId,
                election_event_id: electionEventId,
                election_id: electionId,
                results_event_id: resultsEventId,
            },
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    useEffect(() => {
        if (results && candidatesList.length) {
            const temp: Array<Sequent_Backend_Candidate_Extended> | undefined = candidatesList?.map(
                (candidate, index) => {
                    let candidateResult = results.find((r) => r.candidate_id === candidate.id)

                    return {
                        ...candidate,
                        rowId: index,
                        id: candidate.id || "",
                        name: candidate.name,
                        status: candidate.status || "",
                        cast_votes: candidateResult?.cast_votes,
                        cast_votes_percent: candidateResult?.cast_votes_percent,
                        winning_position: candidateResult?.winning_position,
                    }
                }
            )

            setResultsData(temp)
            setAreaResultsData(temp)
        }
    }, [results, candidatesList])

    const {data: resultsArea} = useGetList<Sequent_Backend_Results_Area_Contest_Candidate>(
        "sequent_backend_results_area_contest_candidate",
        {
            pagination: {page: 1, perPage: 9999},
            filter: {
                contest_id: contestId,
                tenant_id: tenantId,
                election_event_id: electionEventId,
                election_id: electionId,
                area_id: selectedArea,
                results_event_id: resultsEventId,
            },
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    useEffect(() => {
        if (resultsArea && candidatesList.length) {
            const temp: Array<Sequent_Backend_Candidate_Extended> | undefined = candidatesList?.map(
                (candidate, index) => {
                    let candidateResult = resultsArea.find((r) => r.candidate_id === candidate.id)

                    return {
                        ...candidate,
                        rowId: index,
                        id: candidate.id || "",
                        name: candidate.name,
                        status: candidate.status || "",
                        cast_votes: candidateResult?.cast_votes,
                        cast_votes_percent: candidateResult?.cast_votes_percent,
                        winning_position: candidateResult?.winning_position,
                    }
                }
            )

            setAreaResultsData(temp)
        }
    }, [resultsArea, candidatesList])

    const tabClicked = (area: Sequent_Backend_Area_Contest, index: number) => {
        setValue(index + 1)
        setSelectedArea(area.area_id)
    }

    const tabGlobalClicked = () => {
        setValue(0)
    }

    let documents: IResultDocuments | null = useMemo(
        () =>
            (!!contestId &&
                !!selectedArea &&
                !!resultsContests &&
                resultsContests[0]?.contest_id === contestId &&
                resultsContests[0]?.area_id === selectedArea &&
                (resultsContests[0]?.documents as IResultDocuments | null)) ||
            null,
        [
            contestId,
            selectedArea,
            resultsContests,
            resultsContests?.[0]?.contest_id,
            resultsContests?.[0]?.area_id,
        ]
    )

    return (
        <>
            <Box
                sx={{
                    borderBottom: 1,
                    borderColor: "divider",
                    display: "flex",
                    flexDirection: "row",
                    justifyContent: "flex-start",
                    alignItems: "center",
                }}
            >
                <Typography variant="body2" component="div" sx={{width: "80px"}}>
                    {t("electionEventScreen.stats.areas")}.{" "}
                </Typography>
                <Tabs value={value} sx={{flex: 1}}>
                    <Tab label={t("tally.common.global")} onClick={() => tabGlobalClicked()} />

                    {contestAreas?.map((area, index) => {
                        return (
                            <Tab
                                key={index}
                                label={area.name}
                                onClick={() => tabClicked(area, index)}
                            />
                        )
                    })}
                </Tabs>
                {documents && electionEventId ? (
                    <ExportElectionMenu
                        documents={documents}
                        electionEventId={electionEventId}
                        itemName={contest?.name ?? "contest"}
                    />
                ) : null}
            </Box>

            <CustomTabPanel index={0} value={value}>
                <TallyResultsGlobalCandidates />
            </CustomTabPanel>
            {contestAreas?.map((area, index) => (
                <CustomTabPanel key={index} index={index + 1} value={value}>
                    <TallyResultsCandidates
                        electionEventId={contest?.election_event_id}
                        tenantId={contest?.tenant_id}
                        electionId={contest?.election_id}
                        contestId={contest?.id}
                        areaId={selectedArea}
                        resultsEventId={resultsEventId}
                    />
                </CustomTabPanel>
            ))}
        </>
    )
}

interface TabPanelProps {
    children?: reactI18next.ReactI18NextChild | Iterable<reactI18next.ReactI18NextChild>
    index: number
    value: number | null
}

export function CustomTabPanel(props: TabPanelProps) {
    const {children, value, index, ...other} = props

    return (
        <div role="tabpanel" hidden={value !== index} {...other}>
            {value === index && <Box>{children}</Box>}
        </div>
    )
}

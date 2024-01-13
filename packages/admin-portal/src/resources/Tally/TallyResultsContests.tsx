// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useState} from "react"
import {Identifier, RaRecord, useGetList} from "react-admin"

import {Sequent_Backend_Contest, Sequent_Backend_Results_Contest} from "../../gql/graphql"
import {Box, Tab, Tabs, Typography} from "@mui/material"
import * as reactI18next from "react-i18next"
import {TallyResultsContestAreas} from "./TallyResultsContestAreas"
import {ExportElectionMenu} from "@/components/tally/ExportElectionMenu"
import { SettingsContext } from "@/providers/SettingsContextProvider"
import { IResultDocuments } from "@/types/results"

interface TallyResultsContestProps {
    areas: RaRecord<Identifier>[] | undefined
    electionId: string | null
    electionEventId: string | null
    tenantId: string | null
    resultsEventId: string | null
}

export const TallyResultsContest: React.FC<TallyResultsContestProps> = (props) => {
    const {areas, electionId, electionEventId, tenantId, resultsEventId} = props
    const [value, setValue] = React.useState<number | null>(0)
    const [contestsData, setContestsData] = useState<Array<Sequent_Backend_Contest>>([])
    const [contestId, setContestId] = useState<string | null>()
    const {globalSettings} = useContext(SettingsContext)

    const {t} = reactI18next.useTranslation()
    const [electionData, setElectionData] = useState<string | null>(null)
    const [electionEventData, setElectionEventData] = useState<string | null>(null)
    const [tenantData, setTenantData] = useState<string | null>(null)
    const [areasData, setAreasData] = useState<RaRecord<Identifier>[]>()

    // console.log("TallyResultsContest :: contestsData", contestsData)

    const {data: resultsContests} = useGetList<Sequent_Backend_Results_Contest>(
        "sequent_backend_results_contest",
        {
            pagination: {page: 1, perPage: 1},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventId,
                results_event_id: resultsEventId,
                election_id: electionId,
                contest_id: contestId,
            },
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: contests} = useGetList<Sequent_Backend_Contest>(
        "sequent_backend_contest",
        {
            filter: {
                election_id: electionData,
                tenant_id: tenantData,
                election_event_id: electionEventData,
            },
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    useEffect(() => {
        if (electionId) {
            setElectionData(electionId)
        }
    }, [electionId])

    useEffect(() => {
        if (electionEventId) {
            setElectionEventData(electionEventId)
        }
    }, [electionEventId])

    useEffect(() => {
        if (tenantId) {
            setTenantData(tenantId)
        }
    }, [tenantId])

    useEffect(() => {
        if (areas) {
            setAreasData(areas)
        }
    }, [areas])

    useEffect(() => {
        if (electionData) {
            setContestsData(contests || [])
            tabClicked(contests?.[0]?.id, 0)
        }
    }, [electionData, contests])

    interface TabPanelProps {
        children?: reactI18next.ReactI18NextChild | Iterable<reactI18next.ReactI18NextChild>
        index: number
        value: number | null
    }

    function CustomTabPanel(props: TabPanelProps) {
        const {children, value, index, ...other} = props

        return (
            <div role="tabpanel" hidden={value !== index} {...other}>
                {value === index && <Box>{children}</Box>}
            </div>
        )
    }

    const tabClicked = (id: string, index: number) => {
        setValue(index)
        setContestId(id)
    }
    let documents = !!contestId && !!resultsContests && resultsContests[0]?.contest_id === contestId && ( resultsContests[0]?.documents as IResultDocuments | null)

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
                    {t("electionEventScreen.stats.contests")}.{" "}
                </Typography>
                <Tabs value={value} sx={{flex: 1}}>
                    {contestsData?.map((contest, index) => (
                        <Tab
                            key={index}
                            label={contest.name}
                            onClick={() => tabClicked(contest.id, index)}
                        />
                    ))}
                </Tabs>
                {
                    documents && electionEventId
                    ? <ExportElectionMenu
                        documents={documents}
                        electionEventId={electionEventId}
                        item={""}
                    />
                    : null
                }
            </Box>

            {contestsData?.map((contest, index) => (
                <CustomTabPanel key={index} index={index} value={value}>
                    <TallyResultsContestAreas
                        areas={areasData}
                        contestId={contestId || null}
                        electionId={contest?.election_id}
                        electionEventId={contest?.election_event_id}
                        tenantId={contest?.tenant_id}
                        resultsEventId={resultsEventId}
                    />
                </CustomTabPanel>
            ))}
        </>
    )
}

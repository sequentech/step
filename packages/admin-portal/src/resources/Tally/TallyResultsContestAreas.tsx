// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Identifier, RaRecord, useGetList, useGetOne} from "react-admin"

import {Sequent_Backend_Area_Contest, Sequent_Backend_Contest} from "../../gql/graphql"
import {Box, Tabs, Tab, Typography} from "@mui/material"
import * as reactI18next from "react-i18next"
import {TallyResultsGlobalCandidates} from "./TallyResultsGlobalCandidates"
import {TallyResultsCandidates} from "./TallyResultsCandidates"
import {ExportElectionMenu} from "@/components/tally/ExportElectionMenu"

interface TallyResultsContestAreasProps {
    areas: RaRecord<Identifier>[] | undefined
    contestId: string | null
    electionId: string | null
    electionEventId: string | null
    tenantId: string | null
    resultsEventId: string | null
}

export const TallyResultsContestAreas: React.FC<TallyResultsContestAreasProps> = (props) => {
    const {areas, contestId, electionEventId, tenantId, resultsEventId} = props
    const {t} = reactI18next.useTranslation()

    const [value, setValue] = React.useState<number | null>(null)
    const [areasData, setAreasData] = useState<Array<Sequent_Backend_Area_Contest>>([])
    const [areaContestId, setAreaContestId] = useState<string | null>()
    const [selectedArea, setSelectedArea] = useState<string | null>()

    const {data: contestAreas} = useGetList<Sequent_Backend_Area_Contest>(
        "sequent_backend_area_contest",
        {
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventId,
                contest_id: contestId,
            },
        },
        {
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

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

    useEffect(() => {
        tabGlobalClicked()
    }, [])

    useEffect(() => {
        if (contestId) {
            setAreasData(contestAreas || [])
        }
    }, [contestId, contestAreas])

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

    const tabClicked = (area: Sequent_Backend_Area_Contest, index: number) => {
        setValue(index + 1)
        setAreaContestId(area.id)
        setSelectedArea(area.area_id)
    }

    const tabGlobalClicked = () => {
        setValue(0)
    }

    useEffect(() => {
        console.log("TallyResultsContestAreas :: ", value)
    }, [value])

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
                    {areasData?.map((area, index) => {
                        return (
                            <Tab
                                key={index}
                                label={areas?.find((item) => item.id === area.area_id)?.name}
                                onClick={() => tabClicked(area, index)}
                            />
                        )
                    })}
                </Tabs>
                {value !== null ? (
                    <ExportElectionMenu
                        resource={"sequent_backend_results_area_contest"}
                        area={value < 1 ? "all" : areasData?.[value - 1]}
                        areaName={areas?.find((item) => item.id === selectedArea)?.name}
                    />
                ) : null}
            </Box>

            <CustomTabPanel index={0} value={value}>
                <TallyResultsGlobalCandidates
                    electionEventId={contest?.election_event_id}
                    tenantId={contest?.tenant_id}
                    electionId={contest?.election_id}
                    contestId={contest?.id}
                    resultsEventId={resultsEventId}
                />
            </CustomTabPanel>
            {areasData?.map((area, index) => (
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

// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Identifier, RaRecord, useGetList, useGetOne} from "react-admin"

import {
    Sequent_Backend_Area,
    Sequent_Backend_Area_Contest,
    Sequent_Backend_Candidate,
    Sequent_Backend_Tally_Session,
} from "../../gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Box, Tabs, Tab} from "@mui/material"
import * as reactI18next from "react-i18next"

interface TallyResultsContestAreasProps {
    areas: RaRecord<Identifier>[] | undefined
    contestId: string | null
}

export const TallyResultsContestAreas: React.FC<TallyResultsContestAreasProps> = (props) => {
    const {areas, contestId} = props
    // const [tenantId] = useTenantStore()
    const [value, setValue] = React.useState<number | null>(null)
    const [areasData, setAreasData] = useState<Array<Sequent_Backend_Area_Contest>>([])
    const [tenantId, setTenantId] = useState<string | null>()
    const [electionEventId, setElectionEventId] = useState<string | null>()
    const [areaContestId, setAreaContestId] = useState<string | null>()

    const {data: contestAreas} = useGetList<Sequent_Backend_Area_Contest>(
        "sequent_backend_area_contest",
        {
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventId,
                contest_id: contestId,
            },
        }
    )

    useEffect(() => {
        setTenantId(localStorage.getItem("selected-results-tenant-id"))
        setElectionEventId(localStorage.getItem("selected-results-election-event-id"))
    }, [contestId])

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

   
    const tabClicked = (areaId: string, index: number) => {        
        localStorage.setItem("selected-results-contest-area-id", areasData?.[index]?.id)
        setValue(index)

        setAreaContestId(areaId)
        localStorage.setItem("selected-results-contest-area-id", areaId)
    }

    return (
        <>
            <Tabs value={value}>
                {areasData?.map((area, index) => {
                    return (
                        <Tab
                            key={index}
                            label={areas?.find((item) => item.id === area.area_id)?.name}
                            onClick={() => tabClicked(area.id, index)}
                        />
                    )
                })}
            </Tabs>
            {areasData?.map((area, index) => (
                <CustomTabPanel key={index} index={index} value={value}>
                    {/* <TallyResultsCandidates electionId={electionId} contestId={contest.id} /> */}
                </CustomTabPanel>
            ))}
        </>
    )
}

// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetList, useGetOne} from "react-admin"

import {
    Sequent_Backend_Area_Contest,
    Sequent_Backend_Candidate,
    Sequent_Backend_Tally_Session,
} from "../../gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import { Box, Tabs,Tab } from '@mui/material'
import * as reactI18next from "react-i18next"

// interface TallyResultsContestAreasProps {
//     contestId: string
//     electionId: string
// }

export const TallyResultsContestAreas: React.FC = () => {
    // const {contestId, electionId} = props
    // const [tenantId] = useTenantStore()
    const [value, setValue] = React.useState(0)
    const [areasData, setAreasData] = useState<Array<Sequent_Backend_Area_Contest>>([])
    const [tenantId, setTenantId] = useState<string | null>()
    const [electionEventId, setElectionEventId] = useState<string | null>()
    const [contestId, setContestId] = useState<string | null>()

    const {data: areas} = useGetList<Sequent_Backend_Area_Contest>("sequent_backend_area_contest", {
        filter: {
            tenant_id: tenantId, 
            election_event_id: electionEventId,
            contest_id: contestId
        },
    })

    useEffect(() => {
        console.log("TallyResultsContest :: get storage", contestId)
        setTenantId(localStorage.getItem("selected-results-tenant-id"))
        setElectionEventId(localStorage.getItem("selected-results-election-event-id"))
        setContestId(localStorage.getItem("selected-results-contest-id"))
    }, [contestId, electionEventId])

    useEffect(() => {
        if (contestId) {
            setAreasData(areas || [])
        }
    }, [contestId, areas])

        interface TabPanelProps {
            children?: reactI18next.ReactI18NextChild | Iterable<reactI18next.ReactI18NextChild>
            index: number
            value: number
        }

        function CustomTabPanel(props: TabPanelProps) {
            const {children, value, index, ...other} = props

            return (
                <div role="tabpanel" hidden={value !== index} {...other}>
                    {value === index && <Box>{children}</Box>}
                </div>
            )
        }

        const handleChange = (event: React.SyntheticEvent, newValue: number) => {
            localStorage.setItem("selected-results-contest-area-id", areasData?.[newValue]?.id)
            localStorage.setItem("selected-results-contest-tab-id", newValue.toString())
            setValue(newValue)
        }

    return (
        <>
            <Tabs value={value} onChange={handleChange}>
                {areasData?.map((area, index) => (
                    <Tab key={index} label={area.area_id} />
                ))}
            </Tabs>
            {areasData?.map((area, index) => (
                <CustomTabPanel key={index} index={index} value={value}>
                    {/* <TallyResultsCandidates electionId={electionId} contestId={contest.id} /> */}
                </CustomTabPanel>
            ))}
        </>
    )
}

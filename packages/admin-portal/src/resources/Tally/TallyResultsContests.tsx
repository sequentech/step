// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Identifier, RaRecord, useGetList, useGetOne} from "react-admin"

import {Sequent_Backend_Area, Sequent_Backend_Contest, Sequent_Backend_Tally_Session} from "../../gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Box, Tab, Tabs} from "@mui/material"
import * as reactI18next from "react-i18next"
import {TallyResultsCandidates} from "./TallyResultsCandidates"
import { TallyResultsContestAreas } from './TallyResultsContestAreas'

interface TallyResultsContestProps {
    areas: RaRecord<Identifier>[] | undefined
    electionId: string | null
}

export const TallyResultsContest: React.FC<TallyResultsContestProps> = (props) => {
    const {areas, electionId} = props
    // const [tenantId] = useTenantStore()
    const [value, setValue] = React.useState<number | null>(null)
    const [contestsData, setContestsData] = useState<Array<Sequent_Backend_Contest>>([])
    // const [electionId, setElectionId] = useState<string | null>()
    const [electionEventId, setElectionEventId] = useState<string | null>()
    const [tenantId, setTenantId] = useState<string | null>()
    const [contestId, setContestId] = useState<string | null>()

    // const {data: election} = useGetOne("sequent_backend_election", {
    //     id: electionId,
    //     meta: {tenant_id: tenantId},
    // })

    const {data: contests} = useGetList<Sequent_Backend_Contest>("sequent_backend_contest", {
        filter: {
            election_id: electionId,
            tenant_id: tenantId,
            election_event_id: electionEventId,
        },
    })

    useEffect(() => {
        setTenantId(localStorage.getItem("selected-results-tenant-id"))
        setElectionEventId(localStorage.getItem("selected-results-election-event-id"))
    }, [])

    useEffect(() => {
        if (electionId) {
            setContestsData(contests || [])
        }
    }, [electionId, contests])
    

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
        // localStorage.setItem("selected-results-contest-tab-id", index.toString())
        setValue(index)
        setContestId(id)
    }

    return (
        <>
            <Tabs value={value} >
                {contestsData?.map((contest, index) => (
                    <Tab key={index} label={contest.name} onClick={() => tabClicked(contest.id, index)} />
                ))}
            </Tabs>
            {contestsData?.map((contest, index) => (
                <CustomTabPanel key={index} index={index} value={value}>
                    <TallyResultsContestAreas areas={areas} contestId={contestId || null}/>
                </CustomTabPanel>
            ))}
        </>
    )
}

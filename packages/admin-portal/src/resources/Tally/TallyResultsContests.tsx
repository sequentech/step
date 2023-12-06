// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {useGetList} from "react-admin"

import {Sequent_Backend_Contest, Sequent_Backend_Tally_Session} from "../../gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Box, Tab, Tabs} from "@mui/material"
import * as reactI18next from "react-i18next"
import {TallyResultsCandidates} from "./TallyResultsCandidates"

interface TallyResultsContestProps {
    electionId: string
    tally: Sequent_Backend_Tally_Session | undefined
}

export const TallyResultsContest: React.FC<TallyResultsContestProps> = (props) => {
    const {electionId, tally} = props
    const [tenantId] = useTenantStore()
    const [value, setValue] = React.useState(0)
    const [contestsData, setContestsData] = useState<Array<Sequent_Backend_Contest>>([])

    const {data: contests} = useGetList<Sequent_Backend_Contest>("sequent_backend_contest", {
        filter: {
            election_id: electionId,
            tenant_id: tenantId,
            election_event_id: tally?.election_event_id,
        },
    })

    useEffect(() => {
        if (electionId && tally) {
            setContestsData(contests || [])
        }
    }, [electionId, tally, contests])

    useEffect(() => {
        const selectedTabId = localStorage.getItem("selected-results-contest-tab-id")
        if (selectedTabId) {
            setValue(parseInt(selectedTabId))
        } else {
            localStorage.setItem("selected-results-contest-tab-id", "0")
            setValue(0)
        }
    }, [tally])

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
        localStorage.setItem("selected-results-contest-tab-id", newValue.toString())
        setValue(newValue)
    }

    return (
        <>
            <Tabs value={value} onChange={handleChange}>
                {contestsData?.map((contest, index) => (
                    <Tab key={index} label={contest.name} />
                ))}
            </Tabs>
            {contestsData?.map((contest, index) => (
                <CustomTabPanel key={index} index={index} value={value}>
                    <TallyResultsCandidates contestId={contest.id} tally={tally} />
                </CustomTabPanel>
            ))}
        </>
    )
}

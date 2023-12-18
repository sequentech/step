// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {Identifier, RaRecord, useGetList} from "react-admin"

import {Sequent_Backend_Contest} from "../../gql/graphql"
import {Box, Tab, Tabs} from "@mui/material"
import * as reactI18next from "react-i18next"
import {TallyResultsContestAreas} from "./TallyResultsContestAreas"

interface TallyResultsContestProps {
    areas: RaRecord<Identifier>[] | undefined
    electionId: string | null
    electionEventId: string | null
    tenantId: string | null
    resultsEventId: string | null
}

export const TallyResultsContest: React.FC<TallyResultsContestProps> = (props) => {
    const {areas, electionId, electionEventId, tenantId, resultsEventId} = props
    const [value, setValue] = React.useState<number | null>(null)
    const [contestsData, setContestsData] = useState<Array<Sequent_Backend_Contest>>([])
    const [contestId, setContestId] = useState<string | null>()

    const [electionData, setElectionData] = useState<string | null>(null)
    const [electionEventData, setElectionEventData] = useState<string | null>(null)
    const [tenantData, setTenantData] = useState<string | null>(null)
    const [areasData, setAreasData] = useState<RaRecord<Identifier>[]>()

    // console.log("TallyResultsContest :: contestsData", contestsData)

    const {data: contests} = useGetList<Sequent_Backend_Contest>("sequent_backend_contest", {
        filter: {
            election_id: electionData,
            tenant_id: tenantData,
            election_event_id: electionEventData,
        },
    })

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
        console.log("TallyResultsContest :: in effect contestsData", contests)
        if (electionData) {
            setContestsData(contests || [])
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

    return (
        <>
            <Tabs value={value}>
                {contestsData?.map((contest, index) => (
                    <Tab
                        key={index}
                        label={contest.name}
                        onClick={() => tabClicked(contest.id, index)}
                    />
                ))}
            </Tabs>
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

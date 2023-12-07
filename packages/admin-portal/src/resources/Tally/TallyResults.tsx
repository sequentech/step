// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {memo, useEffect, useState} from "react"
import {
    useGetOne,
    useGetMany,
    TabbedShowLayout,
    RaRecord,
    Identifier,
    useGetList,
} from "react-admin"

import {
    Sequent_Backend_Area,
    Sequent_Backend_Election,
    Sequent_Backend_Tally_Session,
} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {TallyResultsContest} from "./TallyResultsContests"
import {Box, Tab, Tabs} from "@mui/material"
import {ReactI18NextChild} from "react-i18next"

// interface TallyResultsProps {
//     tally: Sequent_Backend_Tally_Session | undefined
// }

const TallyResults: React.FC = () => {
    const [tallyId] = useElectionEventTallyStore()
    const [value, setValue] = React.useState<number | null>(null)
    const [electionsData, setElectionsData] = useState<Array<Sequent_Backend_Election>>([])
    const [electionId, setElectionId] = useState<string | null>(null)

    const {data} = useGetOne<Sequent_Backend_Tally_Session>("sequent_backend_tally_session", {
        id: tallyId,
    })

    const {data: areas} = useGetList<RaRecord<Identifier>>("sequent_backend_area", {
        filter: {
            tenant_id: data?.tenant_id,
            election_event_id: data?.election_event_id,
        },
    })

    const {data: elections} = useGetMany<Sequent_Backend_Election>("sequent_backend_election", {
        ids: data?.election_ids || [],
    })

    useEffect(() => {
        if (elections) {
            console.log("TallyResults :: elections", elections)
            setElectionsData(elections)
        }
    }, [elections])

    interface TabPanelProps {
        children?: ReactI18NextChild | Iterable<ReactI18NextChild>
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

    const handleChange = (event: React.SyntheticEvent, newValue: number) => {}

    const tabClicked = (id: string, index: number) => {
        setElectionId(id)
        // localStorage.setItem("selected-results-election-tab-id", index.toString())
        
        localStorage.setItem("selected-results-election-id", id)
        localStorage.setItem(
            "selected-results-election-event-id",
            electionsData?.[index]?.election_event_id
        )
        localStorage.setItem("selected-results-tenant-id", electionsData?.[index]?.tenant_id)
        
        setValue(index)
    }

    return (
        <>
            <Tabs value={value} onChange={handleChange}>
                {electionsData?.map((election, index) => (
                    <Tab
                        key={index}
                        label={election.name}
                        onClick={() => tabClicked(election.id, index)}
                    />
                ))}
            </Tabs>
            {electionsData?.map((election, index) => (
                <CustomTabPanel key={index} index={index} value={value}>
                    <TallyResultsContest areas={areas} electionId={electionId} />
                </CustomTabPanel>
            ))}
        </>
    )
}

export default memo(TallyResults)

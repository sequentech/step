// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {memo, useEffect, useState} from "react"
import {useGetOne, useGetMany, TabbedShowLayout, RaRecord, Identifier} from "react-admin"

import {Sequent_Backend_Election, Sequent_Backend_Tally_Session} from "../../gql/graphql"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {TallyResultsContest} from "./TallyResultsContests"
import {Box, Tab, Tabs} from "@mui/material"
import {ReactI18NextChild} from "react-i18next"

// interface TallyResultsProps {
//     tally: Sequent_Backend_Tally_Session | undefined
// }

const TallyResults: React.FC = () => {
    const [tallyId] = useElectionEventTallyStore()
    const [value, setValue] = React.useState(0)
    const [electionsData, setElectionsData] = useState<Array<Sequent_Backend_Election>>([])

    const {data} = useGetOne<Sequent_Backend_Tally_Session>("sequent_backend_tally_session", {
        id: tallyId,
    })

    useEffect(() => {
        const selectedTabId = localStorage.getItem("selected-results-election-tab-id")
        if (selectedTabId) {
            setValue(parseInt(selectedTabId))
        } else {
            localStorage.setItem("selected-results-election-tab-id", "0")
            setValue(0)
        }
    }, [data])

    const {data: elections} = useGetMany<Sequent_Backend_Election>("sequent_backend_election", {
        ids: data?.election_ids || [],
    })

    useEffect(() => {
        if (elections) {
            console.log("elections in resultas", elections)

            setElectionsData(elections)
        }
    }, [elections])

    interface TabPanelProps {
        children?: ReactI18NextChild | Iterable<ReactI18NextChild>
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
        localStorage.setItem("selected-results-election-tab-id", newValue.toString())
        setValue(newValue)
    }

    return (
        <>
            <Tabs value={value} onChange={handleChange}>
                {electionsData?.map((election, index) => (
                    <Tab key={index} label={election.name} />
                ))}
            </Tabs>
            {electionsData?.map((election, index) => (
                <CustomTabPanel key={index} index={index} value={value}>
                    <TallyResultsContest electionId={election.id} tally={data} />
                </CustomTabPanel>
            ))}
        </>
    )
}

export default memo(TallyResults)

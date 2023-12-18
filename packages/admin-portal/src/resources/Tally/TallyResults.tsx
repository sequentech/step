// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState, memo} from "react"
import {useGetMany, RaRecord, Identifier, useGetList} from "react-admin"

import {Sequent_Backend_Election, Sequent_Backend_Tally_Session} from "../../gql/graphql"
import {TallyResultsContest} from "./TallyResultsContests"
import {Box, Tab, Tabs} from "@mui/material"
import {ReactI18NextChild} from "react-i18next"

interface TallyResultsProps {
    tally: Sequent_Backend_Tally_Session | undefined
    resultsEventId: string | null
}

const TallyResultsMemo: React.MemoExoticComponent<React.FC<TallyResultsProps>> = memo(
    (props: TallyResultsProps): React.JSX.Element => {
        const {tally, resultsEventId} = props

        const [value, setValue] = React.useState<number | null>(null)
        const [electionsData, setElectionsData] = useState<Array<Sequent_Backend_Election>>([])
        const [electionId, setElectionId] = useState<string | null>(null)
        const [data, setData] = useState<Sequent_Backend_Tally_Session | undefined>()
        const [areasData, setAreasData] = useState<RaRecord<Identifier>[]>()

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
            if (tally) {
                setData(tally)
            }
        }, [tally])

        useEffect(() => {
            if (areas) {
                setAreasData(areas)
            }
        }, [areas])

        useEffect(() => {
            if (elections) {
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

        const tabClicked = (id: string, index: number) => {
            setElectionId(id)
            setValue(index)
        }

        return (
            <>
                <Tabs value={value}>
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
                        <TallyResultsContest
                            areas={areasData}
                            electionId={electionId}
                            electionEventId={election.election_event_id}
                            tenantId={election.tenant_id}
                            resultsEventId={resultsEventId}
                        />
                    </CustomTabPanel>
                ))}
            </>
        )
    }
)

TallyResultsMemo.displayName = "TallyResults"

export const TallyResults = TallyResultsMemo

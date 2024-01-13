// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState, memo, useContext, useMemo} from "react"
import {useGetMany, RaRecord, Identifier, useGetList} from "react-admin"

import {
    Sequent_Backend_Election,
    Sequent_Backend_Results_Election,
    Sequent_Backend_Results_Event,
    Sequent_Backend_Tally_Session,
} from "../../gql/graphql"
import {TallyResultsContest} from "./TallyResultsContests"
import {Box, Tab, Tabs, Typography} from "@mui/material"
import {ReactI18NextChild, useTranslation} from "react-i18next"
import {ExportElectionMenu} from "@/components/tally/ExportElectionMenu"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {IResultDocuments} from "@/types/results"

interface TallyResultsProps {
    tally: Sequent_Backend_Tally_Session | undefined
    resultsEventId: string | null
}

const TallyResultsMemo: React.MemoExoticComponent<React.FC<TallyResultsProps>> = memo(
    (props: TallyResultsProps): React.JSX.Element => {
        const {tally, resultsEventId} = props

        const {t} = useTranslation()
        const {globalSettings} = useContext(SettingsContext)
        const [value, setValue] = React.useState<number | null>(0)
        const [electionsData, setElectionsData] = useState<Array<Sequent_Backend_Election>>([])
        const [electionId, setElectionId] = useState<string | null>(null)
        const [data, setData] = useState<Sequent_Backend_Tally_Session | undefined>()
        const [areasData, setAreasData] = useState<RaRecord<Identifier>[]>()

        const {data: areas} = useGetList<RaRecord<Identifier>>(
            "sequent_backend_area",
            {
                filter: {
                    tenant_id: data?.tenant_id,
                    election_event_id: data?.election_event_id,
                },
            },
            {
                refetchOnWindowFocus: false,
                refetchOnReconnect: false,
                refetchOnMount: false,
            }
        )
        const {data: resultsElection} = useGetList<Sequent_Backend_Results_Election>(
            "sequent_backend_results_election",
            {
                pagination: {page: 1, perPage: 1},
                filter: {
                    tenant_id: data?.tenant_id,
                    election_event_id: data?.election_event_id,
                    id: resultsEventId,
                },
            },
            {
                refetchOnWindowFocus: false,
                refetchOnReconnect: false,
                refetchOnMount: false,
            }
        )

        const {data: elections} = useGetMany<Sequent_Backend_Election>(
            "sequent_backend_election",
            {
                ids: data?.election_ids || [],
            },
            {
                refetchOnWindowFocus: false,
                refetchOnReconnect: false,
                refetchOnMount: false,
            }
        )

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
                tabClicked(elections?.[0]?.id, 0)
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

        let documents: IResultDocuments | null = useMemo(
            () =>
                (!!resultsEventId &&
                    !!resultsElection &&
                    resultsElection?.[0]?.id === resultsEventId &&
                    (resultsElection[0]?.documents as IResultDocuments | null)) ||
                null,
            [resultsEventId, resultsElection, resultsElection?.[0]?.id]
        )

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
                        {t("electionEventScreen.stats.elections")}.{" "}
                    </Typography>
                    <Tabs value={value} sx={{flex: 1}}>
                        {electionsData?.map((election, index) => (
                            <Tab
                                key={index}
                                label={election.name}
                                onClick={() => tabClicked(election.id, index)}
                            />
                        ))}
                    </Tabs>
                    {documents ? (
                        <ExportElectionMenu
                            documents={documents}
                            electionEventId={data?.election_event_id}
                            itemName={resultsElection?.[0]?.name ?? "election"}
                        />
                    ) : null}
                </Box>
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

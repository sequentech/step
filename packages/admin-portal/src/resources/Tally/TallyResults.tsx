// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState, memo, useMemo} from "react"
import {RaRecord, Identifier} from "react-admin"

import {
    Sequent_Backend_Election,
    Sequent_Backend_Results_Election,
    Sequent_Backend_Results_Election_Area,
    Sequent_Backend_Tally_Session,
} from "../../gql/graphql"
import {TallyResultsContest} from "./TallyResultsContests"
import {Box, Tab, Tabs, Typography} from "@mui/material"
import {ReactI18NextChild, useTranslation} from "react-i18next"
import {ExportElectionMenu, IResultDocumentsData} from "@/components/tally/ExportElectionMenu"
import {IResultDocuments} from "@/types/results"
import {useAtomValue} from "jotai"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {useKeysPermissions} from "../ElectionEvent/useKeysPermissions"

interface TallyResultsProps {
    tally: Sequent_Backend_Tally_Session | undefined
    resultsEventId: string | null
    loading?: boolean
    onCreateTransmissionPackage: (v: {area_id: string; election_id: string}) => void
}

const TallyResultsMemo: React.MemoExoticComponent<React.FC<TallyResultsProps>> = memo(
    (props: TallyResultsProps): React.JSX.Element => {
        const {tally, resultsEventId, onCreateTransmissionPackage, loading} = props

        const {t} = useTranslation()
        const [value, setValue] = React.useState<number | null>(0)
        const [electionsData, setElectionsData] = useState<Array<Sequent_Backend_Election>>([])
        const [electionId, setElectionId] = useState<string | null>(null)
        const [data, setData] = useState<Sequent_Backend_Tally_Session | undefined>()
        const [areasData, setAreasData] = useState<RaRecord<Identifier>[]>()
        const tallyData = useAtomValue(tallyQueryData)

        const {canExportCeremony} = useKeysPermissions()
        const aliasRenderer = useAliasRenderer()

        const areas: Array<RaRecord<Identifier>> | undefined = useMemo(
            () => tallyData?.sequent_backend_area?.map((area): RaRecord<Identifier> => area),
            [tallyData?.sequent_backend_area]
        )

        const resultsElection: Array<Sequent_Backend_Results_Election> | undefined = useMemo(
            () =>
                tallyData?.sequent_backend_results_election?.filter(
                    (election) => election.election_id === electionId
                ),
            [electionId, tallyData?.sequent_backend_results_election]
        )

        const resultsElectionArea: Array<Sequent_Backend_Results_Election_Area> | undefined =
            useMemo(
                () =>
                    tallyData?.sequent_backend_results_election_area?.filter(
                        (election) => election.election_id === electionId
                    ),
                [electionId, tallyData?.sequent_backend_results_election_area]
            )

        const elections: Array<Sequent_Backend_Election> | undefined = useMemo(
            () =>
                tallyData?.sequent_backend_election
                    ?.filter((election) => data?.election_ids?.includes(election.id))
                    ?.map(
                        (election): Sequent_Backend_Election => ({
                            ...election,
                            contests: [],
                            contests_aggregate: {nodes: []},
                        })
                    ),
            [tallyData?.sequent_backend_election, data?.election_ids]
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

        const currentElection = useMemo(() => {
            return elections?.find((election) => election.id === electionId)
        }, [elections, electionId])

        let documents: IResultDocumentsData | null = useMemo(() => {
            let parsedDocuments: IResultDocuments | null = null
            try {
                const rawDocuments =
                    !!resultsEventId &&
                    !!electionId &&
                    !!resultsElection &&
                    resultsElection?.[0]?.results_event_id === resultsEventId &&
                    resultsElection?.[0]?.election_id === electionId &&
                    (resultsElection[0]?.documents as IResultDocuments | null)
                if (rawDocuments) {
                    // Check if the documents are already an object.
                    // If they are a string, parse them.
                    parsedDocuments =
                        typeof rawDocuments === "string" ? JSON.parse(rawDocuments) : rawDocuments
                }
            } catch (e) {
                console.error("Failed to parse documents JSON string:", e)
                return null // Return null if parsing fails
            }

            return parsedDocuments
                ? {
                      documents: parsedDocuments,
                      name: aliasRenderer(currentElection) ?? "election",
                      class_type: "election",
                  }
                : null
        }, [resultsEventId, resultsElection, resultsElection?.[0]?.id, currentElection])

        let areasDocuments: IResultDocumentsData[] | null = useMemo(
            () =>
                (!!resultsEventId &&
                    !!electionId &&
                    !!resultsElectionArea &&
                    resultsElectionArea
                        .filter(
                            (area) =>
                                area.results_event_id === resultsEventId &&
                                area.election_id == electionId
                        )
                        ?.map((area) => {
                            return {
                                documents: area.documents,
                                name: area.name ?? "area",
                                class_type: "election",
                                class_subtype: "election-area",
                            }
                        })) ||
                null,
            [resultsEventId, resultsElectionArea]
        )

        const documentsList: IResultDocumentsData[] | null = useMemo(() => {
            if (documents && areasDocuments) {
                return [documents, ...areasDocuments]
            }
            if (documents) {
                return [documents]
            }
            if (areasDocuments) {
                return [...areasDocuments]
            }
            return null
        }, [documents, areasDocuments])

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
                    <Tabs value={value} sx={{flex: 1}} variant="scrollable" scrollButtons="auto">
                        {electionsData?.map((election, index) => (
                            <Tab
                                key={index}
                                label={aliasRenderer(election)}
                                onClick={() => tabClicked(election.id, index)}
                            />
                        ))}
                    </Tabs>
                    {documentsList && canExportCeremony && tally?.id ? (
                        <ExportElectionMenu
                            documentsList={documentsList}
                            electionEventId={data?.election_event_id}
                            itemName={aliasRenderer(currentElection) ?? "election"}
                            tallyType={data?.tally_type}
                            electionId={electionId}
                            onCreateTransmissionPackage={onCreateTransmissionPackage}
                            miruExportloading={loading}
                            tallySessionId={tally.id}
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
                            tallySessionId={tally?.id ?? null}
                        />
                    </CustomTabPanel>
                ))}
            </>
        )
    }
)

TallyResultsMemo.displayName = "TallyResults"

export const TallyResults = TallyResultsMemo

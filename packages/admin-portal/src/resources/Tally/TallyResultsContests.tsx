// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {Identifier, RaRecord, useGetList} from "react-admin"

import {Sequent_Backend_Contest, Sequent_Backend_Results_Contest} from "../../gql/graphql"
import {Box, Tab, Tabs, Typography} from "@mui/material"
import * as reactI18next from "react-i18next"
import {TallyResultsContestAreas} from "./TallyResultsContestAreas"
import {ExportElectionMenu, IResultDocumentsData} from "@/components/tally/ExportElectionMenu"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {IResultDocuments} from "@/types/results"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {useAtomValue} from "jotai"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {useKeysPermissions} from "../ElectionEvent/useKeysPermissions"

interface TallyResultsContestProps {
    areas: RaRecord<Identifier>[] | undefined
    electionId: string | null
    electionEventId: string | null
    tenantId: string | null
    resultsEventId: string | null
    tallySessionId: string | null
}

export const TallyResultsContest: React.FC<TallyResultsContestProps> = (props) => {
    const {areas, electionId, electionEventId, tenantId, resultsEventId, tallySessionId} = props
    const [value, setValue] = React.useState<number | null>(0)
    const [contestsData, setContestsData] = useState<Array<Sequent_Backend_Contest>>([])
    const [contestId, setContestId] = useState<string | null>()
    const {globalSettings} = useContext(SettingsContext)

    const {t} = reactI18next.useTranslation()
    const [electionData, setElectionData] = useState<string | null>(null)
    const [electionEventData, setElectionEventData] = useState<string | null>(null)
    const [tenantData, setTenantData] = useState<string | null>(null)
    const [areasData, setAreasData] = useState<RaRecord<Identifier>[]>()
    const tallyData = useAtomValue(tallyQueryData)

    const {canExportCeremony} = useKeysPermissions()

    const resultsContests: Array<Sequent_Backend_Results_Contest> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_results_contest?.filter(
                (areaContest) =>
                    contestId === areaContest.contest_id && electionId === areaContest.election_id
            ),
        [tallyData?.sequent_backend_results_contest, contestId, electionId]
    )

    const contests: Array<Sequent_Backend_Contest> | undefined = useMemo(
        () =>
            tallyData?.sequent_backend_contest
                ?.map(
                    (contest): Sequent_Backend_Contest => ({
                        ...contest,
                        candidates: [],
                        candidates_aggregate: {nodes: []},
                    })
                )
                ?.filter((contest) => electionData === contest.election_id),
        [tallyData?.sequent_backend_contest, electionData]
    )

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
        if (electionData) {
            setContestsData(contests || [])
            if (contests?.[0]?.id) {
                tabClicked(contests?.[0]?.id, 0)
            }
        }
    }, [electionData, contests])

    interface TabPanelProps {
        children?: React.ReactNode | Iterable<React.ReactNode>
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
        if (id) {
            setContestId(id)
        }
    }

    let contestName: string | undefined = useMemo(
        () =>
            (contestId && contests?.find((contest) => contest.id === contestId)?.name) || undefined,
        [contestId, contests]
    )

    let documents: IResultDocumentsData | null = useMemo(() => {
        let parsedDocuments: IResultDocuments | null = null
        try {
            const rawDocuments =
                !!contestId &&
                !!resultsContests &&
                resultsContests[0]?.contest_id === contestId &&
                resultsContests[0]?.documents

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
                  name: contestName ?? "contest",
                  class_type: "contest",
              }
            : null
    }, [contestId, resultsContests, contestName])

    const aliasRenderer = useAliasRenderer()

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
                    {t("electionEventScreen.stats.contests")}.{" "}
                </Typography>
                <Tabs value={value} sx={{flex: 1}} variant="scrollable" scrollButtons="auto">
                    {contestsData?.map((contest, index) => (
                        <Tab
                            key={index}
                            label={aliasRenderer(contest)}
                            onClick={() => tabClicked(contest.id, index)}
                        />
                    ))}
                </Tabs>
                {documents && electionEventId && canExportCeremony && tallySessionId ? (
                    <ExportElectionMenu
                        documentsList={[documents]}
                        electionEventId={electionEventId}
                        itemName={contestName ?? "contest"}
                        tallySessionId={tallySessionId}
                    />
                ) : null}
            </Box>

            {contestsData?.map((contest, index) => (
                <CustomTabPanel key={index} index={index} value={value}>
                    <TallyResultsContestAreas
                        areas={areasData}
                        contestId={contestId || null}
                        electionId={contest?.election_id}
                        electionEventId={contest?.election_event_id}
                        tenantId={contest?.tenant_id}
                        resultsEventId={resultsEventId}
                        tallySessionId={tallySessionId}
                    />
                </CustomTabPanel>
            ))}
        </>
    )
}

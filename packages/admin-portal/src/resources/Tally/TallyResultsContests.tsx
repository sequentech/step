// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
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
import {useSQLQuery} from "@/hooks/useSQLiteDatabase"

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

    const {data: resultsContests} = useSQLQuery(
        "SELECT * FROM results_contest WHERE election_id = ? AND id = ?",
        ["b3f79d05-77b2-4155-8c7e-c5b024db3ac7", "030f3020-780e-4486-a4bd-38d50ec0fc85"],
        {
            databaseUrl: "/results-a98ed291-5111-4201-915d-04adc4af157c.db",
        }
    )

    const {data: contests} = useSQLQuery(
        "SELECT * FROM contest WHERE election_id = ?",
        ["b3f79d05-77b2-4155-8c7e-c5b024db3ac7"],
        {
            databaseUrl: "/results-a98ed291-5111-4201-915d-04adc4af157c.db",
        }
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
            setContestsData((contests as Sequent_Backend_Contest[]) || [])
            if (contests?.[0]?.id) {
                tabClicked(contests?.[0]?.id, 0)
            }
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
        const documents =
            !!contestId &&
            !!resultsContests &&
            resultsContests[0]?.contest_id === contestId &&
            (resultsContests[0]?.documents as IResultDocuments | null)
        return documents
            ? {
                  documents,
                  name: contestName ?? "contest",
                  class_type: "contest",
              }
            : null
    }, [contestId, resultsContests, resultsContests?.[0]?.documents, contestName])
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

// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo, useState} from "react"
import {Identifier, RaRecord, useGetList, useGetOne} from "react-admin"

import {
    Sequent_Backend_Area_Contest,
    Sequent_Backend_Contest,
    Sequent_Backend_Results_Area_Contest,
} from "../../gql/graphql"
import {Box, Tabs, Tab, Typography} from "@mui/material"
import * as reactI18next from "react-i18next"
import {TallyResultsGlobalCandidates} from "./TallyResultsGlobalCandidates"
import {TallyResultsCandidates} from "./TallyResultsCandidates"
import {ExportElectionMenu, IResultDocumentsData} from "@/components/tally/ExportElectionMenu"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {IResultDocuments} from "@/types/results"
import {useAtomValue} from "jotai"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {useKeysPermissions} from "../ElectionEvent/useKeysPermissions"
import {useSQLQuery} from "@/hooks/useSQLiteDatabase"

interface TallyResultsContestAreasProps {
    areas: RaRecord<Identifier>[] | undefined
    contestId: string | null
    electionId: string | null
    electionEventId: string | null
    tenantId: string | null
    resultsEventId: string | null
    tallySessionId: string | null
}

export const TallyResultsContestAreas: React.FC<TallyResultsContestAreasProps> = (props) => {
    const {
        areas,
        contestId,
        electionId,
        electionEventId,
        tenantId,
        resultsEventId,
        tallySessionId,
    } = props
    const {t} = reactI18next.useTranslation()

    const [value, setValue] = React.useState<number>(0)
    const [areasData, setAreasData] = useState<Array<Sequent_Backend_Area_Contest>>([])
    const [areaContestId, setAreaContestId] = useState<string | null>(null)
    const [selectedArea, setSelectedArea] = useState<string | null>(null)
    const {globalSettings} = useContext(SettingsContext)
    const tallyData = useAtomValue(tallyQueryData)
    const [contest, setContest] = useState<Sequent_Backend_Contest | undefined>()

    const {canExportCeremony} = useKeysPermissions()

    const {data: resultsContests} = useSQLQuery(
        "SELECT * FROM results_area_contest WHERE election_id = ? AND contest_id = ? AND area_id = ?",
        [electionId, contestId, selectedArea],
        {
            databaseUrl: "/results-a98ed291-5111-4201-915d-04adc4af157c.db",
        }
    )

    const {data: contestAreas} = useSQLQuery(
        "SELECT * FROM area_contest WHERE contest_id = ?",
        [contestId],
        {
            databaseUrl: "/results-a98ed291-5111-4201-915d-04adc4af157c.db",
        }
    )

    const {data: contestData} = useSQLQuery("SELECT * FROM contest WHERE id = ?", [contestId], {
        databaseUrl: "/results-a98ed291-5111-4201-915d-04adc4af157c.db",
    })

    useEffect(() => {
        if (contestData) {
            setContest({
                ...contestData[0] as Sequent_Backend_Contest,
                candidates: [],
                candidates_aggregate: {nodes: []},
            })
        }
    }, [contestData])

    useEffect(() => {
        tabGlobalClicked()
    }, [])

    useEffect(() => {
        if (contestId) {
            setAreasData((contestAreas as Sequent_Backend_Area_Contest[]) || [])
        }
    }, [contestId, contestAreas])

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

    const tabClicked = (area: Sequent_Backend_Area_Contest, index: number) => {
        setValue(index + 1)
        setAreaContestId(area.id)
        setSelectedArea(area.area_id)
    }

    const tabGlobalClicked = () => {
        setValue(0)
        setAreaContestId(null)
        setSelectedArea(null)
    }

    let documents: IResultDocumentsData | null = useMemo(() => {
        const documents =
            !!contestId &&
            !!selectedArea &&
            !!resultsContests &&
            resultsContests[0]?.contest_id === contestId &&
            resultsContests[0]?.area_id === selectedArea &&
            (resultsContests[0]?.documents as IResultDocuments | null)
        return documents
            ? {
                  documents,
                  name: contest?.name ?? "contest",
                  class_type: "contest-area",
              }
            : null
    }, [
        contestId,
        selectedArea,
        resultsContests,
        resultsContests?.[0]?.contest_id,
        resultsContests?.[0]?.area_id,
        contest?.name,
    ])

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
                    {t("electionEventScreen.stats.areas")}.{" "}
                </Typography>
                <Tabs value={value} sx={{flex: 1}} variant="scrollable" scrollButtons="auto">
                    <Tab label={t("tally.common.global")} onClick={() => tabGlobalClicked()} />
                    {areasData?.map((area, index) => {
                        return (
                            <Tab
                                key={index}
                                label={aliasRenderer(
                                    areas?.find((item) => item.id === area.area_id)
                                )}
                                onClick={() => tabClicked(area, index)}
                            />
                        )
                    })}
                </Tabs>
                {documents && electionEventId && canExportCeremony && tallySessionId ? (
                    <ExportElectionMenu
                        documentsList={[documents]}
                        electionEventId={electionEventId}
                        itemName={contest?.name ?? "contest"}
                        tallySessionId={tallySessionId}
                    />
                ) : null}
            </Box>

            <CustomTabPanel index={0} value={value}>
                <TallyResultsGlobalCandidates
                    electionEventId={contest?.election_event_id}
                    tenantId={contest?.tenant_id}
                    electionId={contest?.election_id}
                    contestId={contest?.id}
                    resultsEventId={resultsEventId}
                />
            </CustomTabPanel>
            {areasData?.map((area, index) => (
                <CustomTabPanel key={index} index={index + 1} value={value}>
                    <TallyResultsCandidates
                        electionEventId={contest?.election_event_id}
                        tenantId={contest?.tenant_id}
                        electionId={contest?.election_id}
                        contestId={contest?.id}
                        areaId={selectedArea}
                        resultsEventId={resultsEventId}
                    />
                </CustomTabPanel>
            ))}
        </>
    )
}

// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
// SPDX-FileCopyrightText: 2023, 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useContext, useEffect, useMemo, useState} from "react"
import {Box, CircularProgress} from "@mui/material"
import {useQuery} from "@apollo/client"
import {BreadCrumbSteps, BreadCrumbStepsVariant} from "@sequentech/ui-essentials"
import styled from "@emotion/styled"
import {Stats} from "./Stats"
import {useTranslation} from "react-i18next"
import {daysBefore, formatDate, getToday} from "../charts/Charts"
import {VotesPerDay} from "../charts/VotesPerDay"
import {VotingChanel, VotersByChannel} from "../charts/VotersByChannel"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {
    CastVotesPerDay,
    GetElectionEventStatsQuery,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
import {useRecordContext} from "react-admin"
import {EVotingStatus, IElectionEventStatistics, IElectionEventStatus} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {GET_ELECTION_EVENT_STATS} from "@/queries/GetElectionEventStats"
import {getLoginUrl} from "@/services/UrlGeneration"

const Container = styled(Box)`
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
`

interface DashboardElectionEventProps {
    refreshRef: any
    onMount: () => void
}

const DashboardElectionEvent: React.FC<DashboardElectionEventProps> = (props) => {
    const {refreshRef, onMount} = props

    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const [selected, setSelected] = useState(0)
    const cardWidth = 470
    const cardHeight = 300
    const endDate = getToday()
    const startDate = daysBefore(endDate, 6)
    const {t} = useTranslation()

    const {
        loading,
        data: dataStats,
        refetch: doRefetch,
    } = useQuery<GetElectionEventStatsQuery>(GET_ELECTION_EVENT_STATS, {
        variables: {
            tenantId,
            electionEventId: record?.id,
            startDate: formatDate(startDate),
            endDate: formatDate(endDate),
        },
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    })

    const stats = dataStats?.election_event?.[0]?.statistics as IElectionEventStatistics | null

    const metrics = {
        eligibleVotersCount: dataStats?.stats?.total_eligible_voters ?? "-",
        votersCount: dataStats?.stats?.total_distinct_voters ?? "-",
        electionsCount: dataStats?.stats?.total_elections ?? "-",
        areasCount: dataStats?.stats?.total_areas ?? "-",
        emailsSentCount: stats?.num_emails_sent ?? "-",
        smsSentCount: stats?.num_sms_sent ?? "-",
    }

    useEffect(() => {
        onMount()
    }, [onMount])

    useEffect(() => {
        if (!record?.status) {
            return
        }
        const status = record.status as IElectionEventStatus
        let data: Array<number> = [0]
        if (status.keys_ceremony_finished) {
            data.push(1)
        }
        if (status.is_published) {
            data.push(2)
        }
        if ([EVotingStatus.OPEN, EVotingStatus.PAUSED].includes(status.voting_status)) {
            data.push(3)
        }
        if (EVotingStatus.CLOSED === status.voting_status) {
            data.push(4)
        }
        if (status.tally_ceremony_finished) {
            data.push(5)
        }
        setSelected(Math.max(...data))
    }, [record?.status])

    const loginUrl = useMemo(() => {
        return getLoginUrl(globalSettings.VOTING_PORTAL_URL, tenantId ?? "", record?.id ?? "")
    }, [globalSettings.VOTING_PORTAL_URL, tenantId, record?.id])

    if (loading) {
        return <CircularProgress />
    }

    return (
        <>
            <Box sx={{width: 1024, marginX: "auto"}}>
                <BreadCrumbSteps
                    labels={[
                        "electionEventBreadcrumbSteps.created", // 0
                        "electionEventBreadcrumbSteps.keys", // 1
                        "electionEventBreadcrumbSteps.publish", // 2
                        "electionEventBreadcrumbSteps.started", // 3
                        "electionEventBreadcrumbSteps.ended", // 4
                        "electionEventBreadcrumbSteps.results", // 5
                    ]}
                    selected={selected}
                    variant={BreadCrumbStepsVariant.Circle}
                    colorPreviousSteps={true}
                />

                <Box>
                    <button
                        ref={refreshRef}
                        onClick={() => {
                            doRefetch()
                        }}
                        style={{display: "none"}}
                    />

                    <Stats metrics={metrics} />

                    <Container>
                        <VotesPerDay
                            data={(dataStats?.stats?.votes_per_day as CastVotesPerDay[]) ?? null}
                            width={cardWidth}
                            height={cardHeight}
                            endDate={endDate}
                        />
                        <VotersByChannel
                            data={[
                                {
                                    channel: VotingChanel.Online,
                                    count: dataStats?.stats?.total_distinct_voters ?? 0,
                                },
                                {
                                    channel: VotingChanel.Paper,
                                    count: 0,
                                },
                                {
                                    channel: VotingChanel.Telephone,
                                    count: 0,
                                },
                                {
                                    channel: VotingChanel.Postal,
                                    count: 0,
                                },
                            ]}
                            width={cardWidth}
                            height={cardHeight}
                        />
                    </Container>
                </Box>
                <Box
                    sx={{
                        display: "flex",
                        justifyContent: "center",
                        gap: "20px",
                        paddingTop: "10px",
                    }}
                >
                    <a href={loginUrl ?? ""} target="_blank">
                        {t("dashboard.voterLoginURL")}
                    </a>
                    <p>|</p>
                    <a href={loginUrl ? loginUrl.replace("/login", "/enroll") : ""} target="_blank">
                        {t("dashboard.voterEnrollURL")}
                    </a>
                </Box>
            </Box>
        </>
    )
}

export default DashboardElectionEvent

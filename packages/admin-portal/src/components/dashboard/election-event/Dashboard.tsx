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
    ListKeysCeremonyQuery,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Tally_Session,
} from "@/gql/graphql"
import {useGetList, useRecordContext} from "react-admin"
import {EVotingStatus, IElectionEventStatistics, IElectionEventStatus} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {GET_ELECTION_EVENT_STATS} from "@/queries/GetElectionEventStats"
import {getAuthUrl} from "@/services/UrlGeneration"
import {ListIpAddress} from "@/resources/ElectionEvent/ListIpAddress"
import {IPermissions} from "@/types/keycloak"
import {AuthContext} from "@/providers/AuthContextProvider"
import {LIST_KEYS_CEREMONY} from "@/queries/ListKeysCeremonies"
import {IKeysCeremonyExecutionStatus} from "@/services/KeyCeremony"
import {ETallyType} from "@/types/ceremonies"

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
    const authContext = useContext(AuthContext)
    const userTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone

    const isTrustee = authContext.isAuthorized(true, tenantId, IPermissions.TRUSTEE_CEREMONY)
    const showIpAdresses = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ADMIN_IP_ADDRESS_VIEW
    )

    // Ensure required parameters are set before running the query.
    const canQuery = Boolean(tenantId && record?.id)

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
            userTimezone,
        },
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        skip: !canQuery,
    })

    const {data: keysCeremonies} = useQuery<ListKeysCeremonyQuery>(LIST_KEYS_CEREMONY, {
        variables: {
            tenantId: tenantId,
            electionEventId: record?.id,
        },
        skip: !canQuery,
        context: {
            headers: {
                "x-hasura-role": isTrustee
                    ? IPermissions.TRUSTEE_CEREMONY
                    : IPermissions.ADMIN_CEREMONY,
            },
        },
    })
    const keysCeremonyIds = useMemo(
        () => keysCeremonies?.list_keys_ceremony?.items?.map((ceremony) => ceremony.id) ?? [],
        [keysCeremonies?.list_keys_ceremony?.items]
    )

    const {data: tallySessions} = useGetList<Sequent_Backend_Tally_Session>(
        "sequent_backend_tally_session",
        {
            pagination: {page: 1, perPage: 9999},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: record?.id,
                keys_ceremony_id: {
                    format: "hasura-raw-query",
                    value: {_in: keysCeremonyIds},
                },
            },
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
            enabled: canQuery,
        }
    )

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
        let finishedKeysCeremonies = keysCeremonies?.list_keys_ceremony?.items?.find(
            (ceremony) =>
                (ceremony.execution_status as IKeysCeremonyExecutionStatus) ===
                IKeysCeremonyExecutionStatus.SUCCESS
        )
        let finishedTallySessions = tallySessions?.find(
            (tallySession) =>
                tallySession.is_execution_completed &&
                tallySession.tally_type !== ETallyType.INITIALIZATION_REPORT
        )
        if (finishedKeysCeremonies) {
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
        if (finishedTallySessions) {
            data.push(5)
        }
        setSelected(Math.max(...data))
    }, [record?.status, keysCeremonies?.list_keys_ceremony?.items, tallySessions])

    const loginUrl = useMemo(() => {
        return getAuthUrl(
            globalSettings.VOTING_PORTAL_URL,
            tenantId ?? "",
            record?.id ?? "",
            "login"
        )
    }, [globalSettings.VOTING_PORTAL_URL, tenantId, record?.id])

    const enrollUrl = useMemo(() => {
        return getAuthUrl(
            globalSettings.VOTING_PORTAL_URL,
            tenantId ?? "",
            record?.id ?? "",
            "enroll"
        )
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
                    {showIpAdresses && record?.id && <ListIpAddress electionEventId={record.id} />}
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
                    <a href={enrollUrl ?? ""} target="_blank">
                        {t("dashboard.voterEnrollURL")}
                    </a>
                    {record?.voting_channels?.kiosk === true && (
                        <>
                            <p>|</p>
                            <a href={enrollUrl ? `${enrollUrl}?kiosk` : ""} target="_blank">
                                {t("dashboard.voterEnrollKioskURL")}
                            </a>
                        </>
                    )}
                </Box>
            </Box>
        </>
    )
}

export default DashboardElectionEvent

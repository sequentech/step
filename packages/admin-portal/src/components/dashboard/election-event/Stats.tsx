// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {Box, CircularProgress} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useGetList, useGetOne, useRecordContext} from "react-admin"
import FenceIcon from "@mui/icons-material/Fence"
import GroupIcon from "@mui/icons-material/Group"
import MarkEmailReadOutlinedIcon from "@mui/icons-material/MarkEmailReadOutlined"
import SmsOutlinedIcon from "@mui/icons-material/SmsOutlined"
import CalendarMonthOutlinedIcon from "@mui/icons-material/CalendarMonthOutlined"
import {GET_ELECTION_EVENT_STATS} from "@/queries/GetElectionEventStats"
import {GetElectionEventStatsQuery, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useQuery} from "@apollo/client"
import styled from "@emotion/styled"
import StatItem from "../StatItem"
import {IElectionEventStatistics} from "@/types/CoreTypes"
import {SettingsContext} from "@/providers/SettingsContextProvider"

const CardList = styled(Box)`
    display: flex;
    width: 100%;
    justify-content: space-between;
    margin: 20px 0;
`

export default function Stats({
    electionEventId,
    electionId,
}: {
    electionEventId: String
    electionId?: String
}) {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)

    const {loading, data: dataStats} = useQuery<GetElectionEventStatsQuery>(
        GET_ELECTION_EVENT_STATS,
        {
            variables: {
                tenantId,
                electionEventId,
                electionId,
            },
            pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        }
    )

    const {data: updatedElectionEvent} = useGetOne<Sequent_Backend_Election_Event>(
        "sequent_backend_election_event",
        {
            id: electionEventId,
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        }
    )

    const {total: totalUsers} = useGetList(
        "user",
        {
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventId,
                election_id: electionId,
            },
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        }
    )

    if (loading) {
        return <CircularProgress />
    }
    const stats = dataStats?.election_event?.statistics as IElectionEventStatistics | null

    const res = {
        castVotes: dataStats?.castVotes?.total_distinct_voters ?? "-",
        elections: dataStats?.elections?.aggregate?.count ?? "-",
        areas: dataStats?.areas?.aggregate?.count ?? "-",
        emailsSent: stats?.num_emails_sent ?? "-",
        smsSent: stats?.num_sms_sent ?? "-",
    }

    const iconSize = 60

    return (
        <CardList>
            <StatItem
                icon={<GroupIcon sx={{fontSize: iconSize}} />}
                count={totalUsers ?? "-"}
                label={t("electionEventScreen.stats.elegibleVoters")}
            ></StatItem>

            {electionId && (
                <StatItem
                    icon={<GroupIcon sx={{fontSize: iconSize}} />}
                    count={res.elections}
                    label={t("electionEventScreen.stats.elections")}
                ></StatItem>
            )}
            <StatItem
                icon={<FenceIcon sx={{fontSize: iconSize}} />}
                count={res.areas}
                label={t("electionEventScreen.stats.areas")}
            ></StatItem>
            <StatItem
                icon={<MarkEmailReadOutlinedIcon sx={{fontSize: iconSize}} />}
                count={res.emailsSent}
                label={t("electionEventScreen.stats.sentEmails")}
            ></StatItem>
            <StatItem
                icon={<SmsOutlinedIcon sx={{fontSize: iconSize}} />}
                count={res.smsSent}
                label={t("electionEventScreen.stats.sentSMS")}
            ></StatItem>
            <StatItem
                icon={<CalendarMonthOutlinedIcon sx={{fontSize: iconSize}} />}
                count={t("electionEventScreen.stats.calendar.scheduled")}
                label={t("electionEventScreen.stats.calendar.title")}
            ></StatItem>
        </CardList>
    )
}

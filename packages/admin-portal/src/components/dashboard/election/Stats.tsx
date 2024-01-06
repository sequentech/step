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
import {GET_ELECTION_STATS} from "@/queries/GetElectionStats"
import {GetElectionStatsQuery, Sequent_Backend_Election} from "@/gql/graphql"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useQuery} from "@apollo/client"
import styled from "@emotion/styled"
import StatItem from "../StatItem"
import {IElectionStatistics} from "@/types/CoreTypes"
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
    electionId: String
}) {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)

    const {loading, data: dataStats} = useQuery<GetElectionStatsQuery>(GET_ELECTION_STATS, {
        variables: {
            tenantId,
            electionEventId,
            electionId,
        },
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    })

    if (loading) {
        return <CircularProgress />
    }
    const stats = dataStats?.election?.[0]?.statistics as IElectionStatistics | null

    const metrics = {
        votersCount: dataStats?.stats?.total_distinct_voters ?? "-",
        eligibleVotersCount: (dataStats?.users as any)?.total?.aggregate?.count ?? "-",
        areasCount: dataStats?.stats?.total_areas ?? "-",
        emailsSentCount: stats?.num_emails_sent ?? "-",
        smsSentCount: stats?.num_sms_sent ?? "-",
    }

    const iconSize = 60

    return (
        <CardList>
            <StatItem
                icon={<GroupIcon sx={{fontSize: iconSize}} />}
                count={metrics.eligibleVotersCount}
                label={t("electionEventScreen.stats.elegibleVoters")}
            ></StatItem>
            <StatItem
                icon={<GroupIcon sx={{fontSize: iconSize}} />}
                count={metrics.votersCount}
                label={t("electionEventScreen.stats.voters")}
            ></StatItem>
            <StatItem
                icon={<FenceIcon sx={{fontSize: iconSize}} />}
                count={metrics.areasCount}
                label={t("electionEventScreen.stats.areas")}
            ></StatItem>
            <StatItem
                icon={<MarkEmailReadOutlinedIcon sx={{fontSize: iconSize}} />}
                count={metrics.emailsSentCount}
                label={t("electionEventScreen.stats.sentEmails")}
            ></StatItem>
            <StatItem
                icon={<SmsOutlinedIcon sx={{fontSize: iconSize}} />}
                count={metrics.smsSentCount}
                label={t("electionEventScreen.stats.sentSMS")}
            ></StatItem>
        </CardList>
    )
}

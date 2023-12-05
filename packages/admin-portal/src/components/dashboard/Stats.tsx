// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Box, CircularProgress} from "@mui/material"
import {useTranslation} from "react-i18next"
import {useGetList, useRecordContext} from "react-admin"
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
import StatItem from "./StatItem"

const CardList = styled(Box)`
    display: flex;
    width: 100%;
    justify-content: space-between;
    margin: 20px 0;
`

export default function Stats({forElection = false}: {forElection?: boolean}) {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()

    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const electionEventId = record.id

    const {loading, data: dataStats} = useQuery<GetElectionEventStatsQuery>(
        GET_ELECTION_EVENT_STATS,
        {
            variables: {
                electionEventId,
                tenantId: record.tenant_id,
            },
        }
    )

    const {total: totalUsers} = useGetList("user", {
        filter: {tenant_id: tenantId, election_event_id: electionEventId},
    })

    if (loading) {
        return <CircularProgress />
    }

    const res = {
        castVotes: dataStats?.castVotes?.aggregate?.count ?? 0,
        elections: dataStats?.elections?.aggregate?.count ?? 0,
        areas: dataStats?.areas?.aggregate?.count ?? 0,
    }

    const iconSize = 60

    return (
        <CardList>
            <StatItem
                icon={<GroupIcon sx={{fontSize: iconSize}} />}
                count={totalUsers ?? 0}
                label={t("electionEventScreen.stats.elegibleVoters")}
            ></StatItem>

            {forElection && (
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
                count={0}
                label={t("electionEventScreen.stats.sentEmails")}
            ></StatItem>
            <StatItem
                icon={<SmsOutlinedIcon sx={{fontSize: iconSize}} />}
                count={0}
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

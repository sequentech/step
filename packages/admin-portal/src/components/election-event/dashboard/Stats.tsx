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
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import StatItem from "@/components/election-event/dashboard/StatItem"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useQuery} from "@apollo/client"
import styled from "@emotion/styled"

const CardList = styled(Box)`
    display: flex;
    width: 100%;
    justify-content: space-between;
    margin: 20px 0;
`

export default function Stats() {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()

    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const electionEventId = record.id

    const {loading, data: dataStats} = useQuery(GET_ELECTION_EVENT_STATS, {
        variables: {
            electionEventId,
            tenantId: record.tenant_id,
        },
    })

    const {data: users, total: totalUsers} = useGetList("user", {
        filter: {tenant_id: tenantId, election_event_id: electionEventId},
    })
  
    console.log(
        "LS -> packages/admin-portal/src/components/election-event/dashboard/Stats.tsx:45 -> totalUsers: ",
        totalUsers
    )
    console.log(
        "LS -> packages/admin-portal/src/components/election-event/dashboard/Stats.tsx:45 -> users: ",
        users
    )

    if (loading) {
        return <CircularProgress />
    }

    const res = {
        castVotes: dataStats.castVotes.aggregate.count,
        elections: dataStats.elections.aggregate.count,
        areas: dataStats.areas.aggregate.count,
    }

    const iconSize = 60

    return (
        <CardList>
            <StatItem
                icon={<GroupIcon sx={{fontSize: iconSize}} />}
                count={-1}
                label={t("electionEventScreen.stats.elegibleVoters")}
            ></StatItem>
            <StatItem
                icon={<GroupIcon sx={{fontSize: iconSize}} />}
                count={res.elections}
                label={t("electionEventScreen.stats.elections")}
            ></StatItem>
            <StatItem
                icon={<FenceIcon sx={{fontSize: iconSize}} />}
                count={res.areas}
                label={t("electionEventScreen.stats.areas")}
            ></StatItem>
            <StatItem
                icon={<MarkEmailReadOutlinedIcon sx={{fontSize: iconSize}} />}
                count={-1}
                label={t("electionEventScreen.stats.sentEmails")}
            ></StatItem>
            <StatItem
                icon={<SmsOutlinedIcon sx={{fontSize: iconSize}} />}
                count={-1}
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

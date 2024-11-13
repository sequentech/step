// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
// SPDX-FileCopyrightText: 2023, 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {Box, CircularProgress} from "@mui/material"

import styled from "@emotion/styled"
import {Stats} from "./Stats"
import {VotesPerDay} from "../charts/VotesPerDay"
import {daysBefore, formatDate, getToday} from "../charts/Charts"
import {VotersByChannel, VotingChanel} from "../charts/VotersByChannel"
import {useRecordContext} from "react-admin"
import {CastVotesPerDay, GetElectionStatsQuery, Sequent_Backend_Election} from "@/gql/graphql"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useQuery} from "@apollo/client"
import {GET_ELECTION_STATS} from "@/queries/GetElectionStats"
import {IElectionStatistics} from "@sequentech/ui-core"
import {useTenantStore} from "@/providers/TenantContextProvider"

const Container = styled(Box)`
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
`

export default function DashboardElection() {
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)
    const record = useRecordContext<Sequent_Backend_Election>()
    const endDate = getToday()
    const startDate = daysBefore(endDate, 6)

    const {loading, data: dataStats} = useQuery<GetElectionStatsQuery>(GET_ELECTION_STATS, {
        variables: {
            tenantId,
            electionEventId: record?.election_event_id,
            electionId: record?.id,
            startDate: formatDate(startDate),
            endDate: formatDate(endDate),
            electionAlias: record?.alias,
        },
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    })

    if (loading) {
        return <CircularProgress />
    }
    const stats = dataStats?.election?.[0]?.statistics as IElectionStatistics | null

    const metrics = {
        eligibleVotersCount: (dataStats?.users as any)?.total?.aggregate?.count ?? "-",
        votersCount: dataStats?.stats?.total_distinct_voters ?? "-",
        areasCount: dataStats?.stats?.total_areas ?? "-",
        emailsSentCount: stats?.num_emails_sent ?? "-",
        smsSentCount: stats?.num_sms_sent ?? "-",
    }

    const cardWidth = 470
    const cardHeight = 250

    return (
        <>
            <Box sx={{width: 1024, marginX: "auto"}}>
                <Box>
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
            </Box>
        </>
    )
}

// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext} from "react"
import {Box, CircularProgress} from "@mui/material"
import {styled} from "@mui/material/styles"
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
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

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
    const userTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone
    const authContext = useContext(AuthContext)

    const showIpAdresses = authContext.isAuthorized(
        true,
        authContext.tenantId,
        IPermissions.ELECTION_IP_ADDRESS_VIEW
    )

    // Ensure required parameters are set before running the query.
    const canQueryStats = Boolean(tenantId && record?.election_event_id && record?.id)

    const {loading, data: dataStats} = useQuery<GetElectionStatsQuery>(GET_ELECTION_STATS, {
        variables: {
            tenantId,
            electionEventId: record?.election_event_id,
            electionId: record?.id,
            startDate: formatDate(startDate),
            endDate: formatDate(endDate),
            electionAlias: record?.alias ?? undefined,
            userTimezone,
        },
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        skip: !canQueryStats,
    })

    if (loading) {
        return <CircularProgress />
    }

    const stats = dataStats?.election?.[0]?.statistics as IElectionStatistics | null

    const metrics = {
        eligibleVotersCount: (dataStats?.users as any)?.count ?? "-",
        votersCount: dataStats?.stats?.total_distinct_voters ?? "-",
        areasCount: dataStats?.stats?.total_areas ?? "-",
        emailsSentCount: stats?.num_emails_sent ?? "-",
        smsSentCount: stats?.num_sms_sent ?? "-",
    }

    const cardWidth = 470
    const cardHeight = 250

    return (
        <Box sx={{width: 1024, marginX: "auto"}} className="dashboard">
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
                {/* NOTE:LOOP showIpAdresses && record?.id && (
                    <ListIpAddress
                        electionEventId={record?.election_event_id}
                        electionId={record?.id}
                    />
                )*/}
            </Box>
        </Box>
    )
}

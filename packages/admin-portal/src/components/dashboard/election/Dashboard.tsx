// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useState} from "react"
import {Box, CircularProgress} from "@mui/material"
import {StandardDashboardContent, StandardDashboardCharts} from "../DashboardContent"
import {Stats} from "./Stats"
import {VotesPerDay} from "../charts/VotesPerDay"
import {
    DEFAULT_VOTES_TIME_SELECTION,
    getVotesBucketCount,
    VotesTimeSelection,
} from "../charts/votesTimeRange"
import {daysBefore, formatDate, getToday} from "../charts/Charts"
import {VotersByChannel} from "../charts/VotersByChannel"
import {toVotersByChannelRows} from "../charts/votersByChannelData"
import {useRecordContext} from "react-admin"
import {CastVotesPerDay, GetElectionStatsQuery, Sequent_Backend_Election} from "@/gql/graphql"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useQuery} from "@apollo/client"
import {GET_ELECTION_STATS} from "@/queries/GetElectionStats"
import {IElectionStatistics, translateFromPresentation} from "@sequentech/ui-core"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useTranslation} from "react-i18next"

export default function DashboardElection() {
    const {i18n} = useTranslation()
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)
    const record = useRecordContext<Sequent_Backend_Election>()
    const [votesTimeSelection, setVotesTimeSelection] = useState<VotesTimeSelection>(
        DEFAULT_VOTES_TIME_SELECTION
    )

    const electionAlias = record
        ? translateFromPresentation(record.presentation, "alias", i18n.language)
        : undefined
    const endDate = getToday()
    const startDate = daysBefore(endDate, 6)
    const userTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone

    // Ensure required parameters are set before running the query.
    const canQueryStats = Boolean(tenantId && record?.election_event_id && record?.id)

    const {
        loading,
        error,
        data: dataStats,
    } = useQuery<GetElectionStatsQuery>(GET_ELECTION_STATS, {
        variables: {
            tenantId,
            electionEventId: record?.election_event_id,
            electionId: record?.id,
            startDate: formatDate(startDate),
            endDate: formatDate(endDate),
            electionAlias: electionAlias ?? undefined,
            userTimezone,
            timeResolution: votesTimeSelection.resolution,
            bucketCount: getVotesBucketCount(votesTimeSelection),
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
        messagesSentCount: {
            WHATSAPP: stats?.num_whatsapp_sent ?? "-",
            VIBER: stats?.num_viber_sent ?? "-",
            MESSENGER: stats?.num_messenger_sent ?? "-",
        },
    }

    const cardHeight = 250

    return (
        <StandardDashboardContent className="dashboard">
            <Box>
                <Stats metrics={metrics} />

                <StandardDashboardCharts>
                    <VotesPerDay
                        data={(dataStats?.stats?.votes_per_day as CastVotesPerDay[]) ?? null}
                        unavailable={!!error}
                        height={cardHeight}
                        selection={votesTimeSelection}
                        onSelectionChange={setVotesTimeSelection}
                    />
                    <VotersByChannel
                        data={toVotersByChannelRows(dataStats?.stats?.voters_by_channel)}
                        height={cardHeight}
                    />
                </StandardDashboardCharts>
            </Box>
        </StandardDashboardContent>
    )
}

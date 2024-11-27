// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo} from "react"
import ElectionStats, {ElectionStatsProps} from "./ElectionStats"
import {useQuery} from "@apollo/client"
import {
    GetElectionEventMonitoringQuery,
    GetElectionMonitoringQuery,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
import {useRecordContext} from "react-admin"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {IPermissions} from "@/types/keycloak"
import {GET_ELECTION_MONITORING} from "@/queries/GetElectionMonitoring"

const MonitoringDashboardElection = () => {
    const record = useRecordContext<Sequent_Backend_Election>()
    const {globalSettings} = useContext(SettingsContext)

    const {
        loading,
        data: dataStats,
        refetch: doRefetch,
    } = useQuery<GetElectionMonitoringQuery>(GET_ELECTION_MONITORING, {
        variables: {
            electionEventId: record?.election_event_id,
            electionId: record?.id,
        },
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        context: {
            headers: {
                "x-hasura-role": IPermissions.MONITORING_DASHBOARD_VIEW_ELECTION,
            },
        },
    })

    const data = useMemo(() => dataStats?.get_election_monitoring, [dataStats])

    const stats: ElectionStatsProps = useMemo(() => {
        return {
            eligibleVotersCount: data?.total_eligible_voters ?? "-",
            enrolledVotersCount: data?.total_enrolled_voters ?? "-",
            votedCount: data?.total_voted ?? "-",
            authenticationStats: {
                authenticatedCount: data?.authentication_stats?.total_authenticated ?? "-",
                notAuthenticatedCount: data?.authentication_stats?.total_not_authenticated ?? "-",
                invalidUsersErrorsCount:
                    data?.authentication_stats?.total_invalid_users_errors ?? "-",
                invalidPasswordErrorsCount:
                    data?.authentication_stats?.total_invalid_password_errors ?? "-",
            },
            approvalStats: {
                approvedCount: data?.approval_stats?.total_approved ?? "-",
                disapprovedCount: data?.approval_stats?.total_disapproved ?? "-",
                manualApprovedCount: data?.approval_stats?.total_manual_approved ?? "-",
                manualDisapprovedCount: data?.approval_stats?.total_manual_disapproved ?? "-",
                automatedApprovedCount: data?.approval_stats?.total_automated_approved ?? "-",
                automatedDisapprovedCount: data?.approval_stats?.total_automated_disapproved ?? "-",
            },
        }
    }, [data])

    return <div>{data && <ElectionStats {...stats} />}</div>
}

export default MonitoringDashboardElection

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect, useMemo} from "react"
import ElectionEventStats, {ElectionEventStatsProps} from "./stats/ElectionEventStats"
import {useQuery} from "@apollo/client"
import {GET_ELECTION_EVENT_MONITORING} from "@/queries/GetElectionEventMonitoring"
import {GetElectionEventMonitoringQuery, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useRecordContext} from "react-admin"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {IPermissions} from "@/types/keycloak"

interface MonitoringDashboardElectionEventProps {
    refreshRef: any
    onMount?: () => void
}

const MonitoringDashboardElectionEvent: React.FC<MonitoringDashboardElectionEventProps> = (
    props
) => {
    const {refreshRef, onMount} = props

    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {globalSettings} = useContext(SettingsContext)

    useEffect(() => {
        onMount?.()
    }, [onMount])

    const {
        loading,
        data: dataStats,
        refetch: doRefetch,
    } = useQuery<GetElectionEventMonitoringQuery>(GET_ELECTION_EVENT_MONITORING, {
        variables: {
            electionEventId: record?.id,
        },
        pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        context: {
            headers: {
                "x-hasura-role": IPermissions.MONITORING_DASHBOARD_VIEW_ELECTION_EVENT,
            },
        },
    })

    const data = useMemo(() => dataStats?.get_election_event_monitoring, [dataStats])

    const stats: ElectionEventStatsProps = useMemo(() => {
        return {
            eligibleVotersCount: data?.total_eligible_voters ?? "-",
            enrolledVotersCount: data?.total_enrolled_voters ?? "-",
            electionsCount: data?.total_elections ?? "-",
            startedVoteCount: data?.total_started_votes ?? "-",
            notStartedVotesCount: data?.total_not_started_votes ?? "-",
            openVotesCount: data?.total_open_votes ?? "-",
            notOpenVotesCount: data?.total_not_open_votes ?? "-",
            closedVotesCount: data?.total_closed_votes ?? "-",
            notClosedVotesCount: data?.total_not_closed_votes ?? "-",
            startCountingVotesCount: data?.total_start_counting_votes ?? "-",
            notStartCountingVotesCount: data?.total_not_start_counting_votes ?? "-",
            initializeCount: data?.total_initialize ?? "-",
            notInitializeCount: data?.total_not_initialize ?? "-",
            genereatedTallyCount: data?.total_genereated_tally ?? "-",
            notGenereatedTallyCount: data?.total_not_genereated_tally ?? "-",
            authenticationStats: {
                authenticatedCount: data?.authentication_stats?.total_authenticated ?? "-",
                notAuthenticatedCount: data?.authentication_stats?.total_not_authenticated ?? "-",
                invalidUsersErrorsCount:
                    data?.authentication_stats?.total_invalid_users_errors ?? "-",
                invalidPasswordErrorsCount:
                    data?.authentication_stats?.total_invalid_password_errors ?? "-",
            },
            votingStats: {
                votedCount: data?.voting_stats?.total_voted ?? "-",
                votedTestsElectionsCount: data?.voting_stats?.total_voted_tests_elections ?? "-",
            },
            approvalStats: {
                approvedCount: data?.approval_stats?.total_approved ?? "-",
                disapprovedCount: data?.approval_stats?.total_disapproved ?? "-",
                manualApprovedCount: data?.approval_stats?.total_manual_approved ?? "-",
                manualDisapprovedCount: data?.approval_stats?.total_manual_disapproved ?? "-",
                automatedApprovedCount: data?.approval_stats?.total_automated_approved ?? "-",
                automatedDisapprovedCount: data?.approval_stats?.total_automated_disapproved ?? "-",
            },
            transmissionStats: {
                transmittedCount: data?.transmission_stats?.total_transmitted_results ?? "-",
                halfTransmittedCount:
                    data?.transmission_stats?.total_half_transmitted_results ?? "-",
                notTransmittedCount: data?.transmission_stats?.total_not_transmitted_results ?? "-",
            },
        }
    }, [data])

    return <div>{data && <ElectionEventStats {...stats} />}</div>
}

export default MonitoringDashboardElectionEvent

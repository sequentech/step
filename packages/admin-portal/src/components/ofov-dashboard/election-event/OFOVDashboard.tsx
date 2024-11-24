// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect} from "react"
import styled from "@emotion/styled"
import {Box} from "@mui/material"
import Stats, {StatsProps} from "../Stats"
import {useQuery} from "@apollo/client"
import {GET_ELECTION_EVENT_MONITORING} from "@/queries/GetElectionEventMonitoring"
import {GetElectionEventMonitoringQuery, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useRecordContext} from "react-admin"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {IPermissions} from "@/types/keycloak"

const Container = styled(Box)`
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
`

interface OVOFDashboardElectionEventProps {
    refreshRef: any
    onMount: () => void
}

const OVOFDashboardElectionEvent: React.FC<OVOFDashboardElectionEventProps> = (props) => {
    const {refreshRef, onMount} = props

    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {globalSettings} = useContext(SettingsContext)

    useEffect(() => {
        onMount()
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
                "x-hasura-role": IPermissions.ADMIN_OFOV_DASHBOARD_VIEW,
            },
        },
    })

    const data = dataStats?.get_election_event_monitoring

    const stats: StatsProps = {
        eligibleVotersCount: data?.total_eligible_voters ?? "-",
        enrolledVotersCount: data?.total_enrolled_voters ?? "-",
        electionsCount: data?.total_elections ?? "-",
        openVotesCount: data?.total_open_votes ?? "-",
        notOpenedVotesCount: data?.total_not_opened_votes ?? "-",
        ClosedVotesCount: data?.total_closed_votes ?? "-",
        notClosedVotesCount: data?.total_not_closed_votes ?? "-",
        startCountingVotesCount: data?.total_start_counting_votes ?? "-",
        notStartCountingVotesCount: data?.total_not_start_counting_votes ?? "-",
        initializeCount: data?.total_initialize ?? "-",
        notInitializeCount: data?.total_not_initialize ?? "-",
        genereatedTallyCount: data?.total_genereated_tally ?? "-",
        notGenereatedTallyCount: data?.total_not_genereated_tally ?? "-",
        transmittedResultsCount: data?.total_transmitted_results ?? "-",
        notTransmittedResultsCounts: data?.total_not_transmitted_results ?? "-",
    }

    return (
        <div>
            <Stats {...stats} />
        </div>
    )
}

export default OVOFDashboardElectionEvent

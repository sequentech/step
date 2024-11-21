// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useContext, useEffect} from "react"
import styled from "@emotion/styled"
import {Box} from "@mui/material"
import Stats from "../Stats"
import {useQuery} from "@apollo/client"
import {GET_ELECTION_EVENT_MONITORING} from "@/queries/GetElectionEventMonitoring"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {useRecordContext} from "react-admin"
import {SettingsContext} from "@/providers/SettingsContextProvider"

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

    // const {
    //     loading,
    //     data: dataStats,
    //     refetch: doRefetch,
    // } = useQuery<ElectionEventMonitoringOutput>(GET_ELECTION_EVENT_MONITORING, {
    //     variables: {
    //         electionEventId: record?.id,
    //     },
    //     pollInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
    // })

    // const stats = {
    //     eligibleVotersCount: dataStats?.total_eligible_voters ?? "-",
    //     enrolledVotersCount: dataStats?.total_enrolled_voters ?? "-",
    //     electionsCount: dataStats?.total_elections ?? "-",
    //     approvedVotersCount: dataStats?.total_approved_voters ?? "-",
    //     disapprovedVotersCount: dataStats?.total_disapproved_voters ?? "-",
    //     disapprovedResons: dataStats?.disapproved_resons ?? [],
    //     openVotesCount: dataStats?.total_open_votes ?? "-",
    //     notOpenedVotesCount: dataStats?.total_not_opened_votes ?? "-",
    //     ClosedVotesCount: dataStats?.total_closed_votes ?? "-",
    //     notClosedVotesCount: dataStats?.total_not_closed_votes ?? "-",
    //     startCountingVotesCount: dataStats?.total_start_counting_votes ?? "-",
    //     notStartCountingVotesCount: dataStats?.total_not_start_counting_votes ?? "-",
    //     initializeCount: dataStats?.total_initialize ?? "-",
    //     notInitializeCount: dataStats?.total_not_initialize ?? "-",
    //     genereatedTallyCount: dataStats?.total_genereated_tally ?? "-",
    //     notGenereatedTallyCount: dataStats?.total_not_genereated_tally ?? "-",
    //     transmittedResultsCount: dataStats?.total_transmitted_results ?? "-",
    //     notTransmittedResultsCounts: dataStats?.total_not_transmitted_results ?? "-",
    // }

    return (
        <div>
            <Stats
                eligibleVotersCount={21}
                enrolledVotersCount={12}
                electionsCount={35}
                approvedVotersCount={12}
                disapprovedVotersCount={4}
                disapprovedResons={[]}
                openVotesCount={5}
                notOpenedVotesCount={29}
                ClosedVotesCount={1}
                notClosedVotesCount={34}
                startCountingVotesCount={0}
                notStartCountingVotesCount={0}
                initializeCount={0}
                notInitializeCount={35}
                genereatedTallyCount={0}
                notGenereatedTallyCount={35}
                transmittedResultsCount={0}
                notTransmittedResultsCounts={0}
            />
        </div>
    )
}

export default OVOFDashboardElectionEvent

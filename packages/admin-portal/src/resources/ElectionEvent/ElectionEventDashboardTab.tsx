// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {Suspense, lazy} from "react"
import {useRecordContext} from "react-admin"
import {Box} from "@mui/material"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {ListIpAddress} from "@/resources/ElectionEvent/ListIpAddress"
import {FailedSealsBanner} from "@/components/dashboard/FailedSealsBanner"
import {monitoringLock} from "@/components/monitoring/useMonitoringPermissions"
import {VoterPortalLinks} from "@/components/dashboard/election-event/VoterPortalLinks"

const DashboardElectionEvent = lazy(() => import("@/components/dashboard/election-event/Dashboard"))
const MonitoringDashboardTab = lazy(() =>
    import("@/components/monitoring/MonitoringDashboardTab").then((m) => ({
        default: m.MonitoringDashboardTab,
    }))
)

export interface ElectionEventDashboardTabProps {
    refreshRef: React.RefObject<HTMLButtonElement | null>
    handleChildMount: () => void
}

export const ElectionEventDashboardTab: React.FC<ElectionEventDashboardTabProps> = ({
    refreshRef,
    handleChildMount,
}) => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const lock = monitoringLock(record)
    return (
        <Suspense fallback={<div>Loading Dashboard...</div>}>
            <Box sx={{width: "100%", minWidth: 0}}>
                {/* A failed seal is an incident the event shows in both monitoring modes. */}
                <FailedSealsBanner
                    electionEventId={record?.id}
                    presentation={record?.presentation}
                />
                <MonitoringDashboardTab
                    electionEventId={record?.id}
                    lock={lock}
                    footer={
                        <>
                            <VoterPortalLinks electionEvent={record} />
                            <ListIpAddress />
                        </>
                    }
                    legacy={
                        <Box sx={{overflowX: "auto"}}>
                            <DashboardElectionEvent
                                refreshRef={refreshRef}
                                onMount={handleChildMount}
                            />
                        </Box>
                    }
                />
            </Box>
        </Suspense>
    )
}

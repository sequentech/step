// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {type ReactNode} from "react"
import {Alert, Box, CircularProgress, Stack} from "@mui/material"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {MONITORING_LIST_DASHBOARDS} from "@/queries/MonitoringListDashboards"
import {
    EMonitoringCapability,
    EMonitoringMode,
    type MonitoringListDashboardsQuery,
    type MonitoringListDashboardsVariables,
} from "./types"
import {EMonitoringLock, useMonitoringPermissions} from "./useMonitoringPermissions"
import {MonitoringProvider, type MonitoringEditorActions} from "./MonitoringProvider"
import {MonitoringDashboard} from "./MonitoringDashboard"

export interface MonitoringDashboardTabProps {
    electionEventId?: string | null
    /** Given on an election's page, which pins the Post. */
    electionId?: string | null
    /** Today's dashboard, shown whenever monitoring is not. */
    legacy: ReactNode
    lock?: EMonitoringLock
    actions?: MonitoringEditorActions
}

/**
 * The Dashboard tab. Monitoring replaces the standard dashboard only for a
 * viewer with `monitoring-view` on an event whose monitoring is configured;
 * anyone else keeps the standard dashboard, and no monitoring request is made
 * for a viewer without the permission.
 */
export function MonitoringDashboardTab({
    electionEventId,
    electionId,
    legacy,
    lock = EMonitoringLock.OPEN,
    actions,
}: MonitoringDashboardTabProps) {
    const {t} = useTranslation()
    const {view, configure} = useMonitoringPermissions(lock)
    const allowed = view === EMonitoringCapability.GRANTED && Boolean(electionEventId)
    const {data, loading, error} = useQuery<
        MonitoringListDashboardsQuery,
        MonitoringListDashboardsVariables
    >(MONITORING_LIST_DASHBOARDS, {
        variables: {electionEventId: electionEventId ?? "", electionId: electionId ?? null},
        skip: !allowed,
    })

    if (!allowed) return <>{legacy}</>
    if (error) {
        return (
            <Stack spacing={2}>
                <Alert severity="warning">{t("monitoring.unavailableAlert")}</Alert>
                {legacy}
            </Stack>
        )
    }
    const list = data?.monitoringListDashboards
    if (loading || !list) {
        return (
            <Box sx={{display: "flex", justifyContent: "center", py: 4}}>
                <CircularProgress aria-label={t("monitoring.loading")} />
            </Box>
        )
    }
    if (list.mode !== EMonitoringMode.CONFIGURED) return <>{legacy}</>
    if (!list.dashboards.length) {
        return <Alert severity="info">{t("monitoring.noDashboards")}</Alert>
    }
    return (
        <MonitoringProvider
            storageKey={`monitoring:${electionEventId}:${electionId ?? ""}`}
            actions={actions}
        >
            <MonitoringDashboard
                electionEventId={electionEventId as string}
                electionId={electionId}
                dashboards={list.dashboards}
                configure={configure}
            />
        </MonitoringProvider>
    )
}

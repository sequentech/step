// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, type ReactNode} from "react"
import {Alert, Box, Button, CircularProgress, Stack} from "@mui/material"
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
import type {IMonitoringEditorApi} from "./editor/api"
import {useMonitoringTabEditor} from "./editor/useMonitoringTabEditor"
import {useMonitoringEditorApi} from "./editor/useMonitoringEditorApi"
import {MonitoringResetToPresetDialog} from "./editor/MonitoringResetToPresetDialog"

export interface MonitoringDashboardTabProps {
    electionEventId?: string | null
    /** Given on an election's page, which pins the Post. */
    electionId?: string | null
    /** Today's dashboard, shown whenever monitoring is not. */
    legacy: ReactNode
    /**
     * Shown under the monitoring dashboards, only while monitoring is
     * configured (also when it has no dashboards). Give `legacy` whatever the
     * standard dashboard needs under it.
     */
    below?: ReactNode
    lock?: EMonitoringLock
    /** Replaces the editor's entry points, which a viewer who may configure gets. */
    actions?: MonitoringEditorActions
    /** The editor's requests; Harvest's actions by default, a fake in stories. */
    editorApi?: IMonitoringEditorApi
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
    below,
    lock = EMonitoringLock.OPEN,
    actions,
    editorApi,
}: MonitoringDashboardTabProps) {
    const {t} = useTranslation()
    const {view, configure} = useMonitoringPermissions(lock)
    const [setupOpen, setSetupOpen] = useState(false)
    const setupApi = useMonitoringEditorApi(electionEventId ?? "")
    const editor = useMonitoringTabEditor({
        electionEventId: electionEventId ?? "",
        electionId,
        configure,
        api: editorApi,
    })
    const allowed = view === EMonitoringCapability.GRANTED && Boolean(electionEventId)
    const {data, loading, error, refetch} = useQuery<
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
    // A new or legacy event has no dashboard whose editor could open the
    // existing preset dialog. Keep that entry available to authorized holders.
    const setup =
        configure === EMonitoringCapability.GRANTED ? (
            <>
                <Box>
                    <Button variant="outlined" onClick={() => setSetupOpen(true)}>
                        {t("monitoring.setup", "Set up monitoring")}
                    </Button>
                </Box>
                <MonitoringResetToPresetDialog
                    open={setupOpen}
                    api={editorApi ?? setupApi}
                    onClose={() => setSetupOpen(false)}
                    onReset={() => {
                        setSetupOpen(false)
                        // useQuery exposes a failed reload through the alert above.
                        void refetch().catch(() => undefined)
                    }}
                />
            </>
        ) : null
    if (list.mode !== EMonitoringMode.CONFIGURED) {
        return (
            <Stack spacing={2}>
                {setup}
                {legacy}
            </Stack>
        )
    }
    if (!list.dashboards.length) {
        return (
            <Stack spacing={2}>
                <Alert severity="info">{t("monitoring.noDashboards")}</Alert>
                {setup}
                {below}
            </Stack>
        )
    }
    return (
        <>
            <MonitoringProvider
                storageKey={`monitoring:${electionEventId}:${electionId ?? ""}`}
                actions={actions ?? editor.actions}
            >
                <MonitoringDashboard
                    electionEventId={electionEventId as string}
                    electionId={electionId}
                    dashboards={list.dashboards}
                    presetTitle={list.preset?.title}
                    configure={configure}
                    refreshSeconds={list.refresh_seconds}
                />
                {editor.element}
            </MonitoringProvider>
            {below}
        </>
    )
}

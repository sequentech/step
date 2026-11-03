// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState, type ReactNode} from "react"
import {Alert, Box, CircularProgress, FormControlLabel, Stack, Switch} from "@mui/material"
import {useMutation, useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {MONITORING_SET_MODE} from "@/queries/MonitoringSetMode"
import {IPermissions} from "@/types/keycloak"
import {MONITORING_LIST_DASHBOARDS} from "@/queries/MonitoringListDashboards"
import {
    EMonitoringCapability,
    EMonitoringMode,
    type MonitoringSetModeMutation,
    type MonitoringListDashboardsQuery,
    type MonitoringListDashboardsVariables,
} from "./types"
import {EMonitoringLock, useMonitoringPermissions} from "./useMonitoringPermissions"
import {MonitoringProvider, type MonitoringEditorActions} from "./MonitoringProvider"
import {MonitoringDashboard} from "./MonitoringDashboard"
import type {IMonitoringEditorApi} from "./editor/api"
import {useMonitoringTabEditor} from "./editor/useMonitoringTabEditor"
import {StandardDashboardContent} from "../dashboard/DashboardContent"

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
    /** Shown in every mode, within the selected dashboard's content bounds. */
    footer?: ReactNode
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
    footer,
    lock = EMonitoringLock.OPEN,
    actions,
    editorApi,
}: MonitoringDashboardTabProps) {
    const {t} = useTranslation()
    const {view, configure} = useMonitoringPermissions(lock)
    const [modeError, setModeError] = useState(false)
    const [setMode, {loading: changingMode}] = useMutation<
        MonitoringSetModeMutation,
        {electionEventId: string; mode: EMonitoringMode}
    >(MONITORING_SET_MODE, {
        context: {headers: {"x-hasura-role": IPermissions.MONITORING_CONFIGURE}},
        refetchQueries: [MONITORING_LIST_DASHBOARDS],
        awaitRefetchQueries: true,
    })
    const changeMode = async (enabled: boolean) => {
        if (!electionEventId || configure !== EMonitoringCapability.GRANTED || changingMode) return
        setModeError(false)
        try {
            await setMode({
                variables: {
                    electionEventId,
                    mode: enabled ? EMonitoringMode.CONFIGURED : EMonitoringMode.LEGACY,
                },
            })
        } catch {
            setModeError(true)
        }
    }
    const editor = useMonitoringTabEditor({
        electionEventId: electionEventId ?? "",
        electionId,
        configure,
        api: editorApi,
    })
    const allowed = view === EMonitoringCapability.GRANTED && Boolean(electionEventId)
    const {data, loading, error} = useQuery<
        MonitoringListDashboardsQuery,
        MonitoringListDashboardsVariables
    >(MONITORING_LIST_DASHBOARDS, {
        variables: {electionEventId: electionEventId ?? "", electionId: electionId ?? null},
        skip: !allowed,
    })

    const standard = (
        <StandardDashboardContent>
            {legacy}
            {footer}
        </StandardDashboardContent>
    )
    if (!allowed) return standard
    if (error) {
        return (
            <Stack spacing={2}>
                <Alert severity="warning">{t("monitoring.unavailableAlert")}</Alert>
                {standard}
            </Stack>
        )
    }
    const list = data?.monitoringListDashboards
    if (loading || !list) {
        return (
            <>
                <Box sx={{display: "flex", justifyContent: "center", py: 4}}>
                    <CircularProgress aria-label={t("monitoring.loading")} />
                </Box>
                {footer}
            </>
        )
    }
    const controls = (
        <Stack spacing={1} sx={{mb: 2}}>
            <FormControlLabel
                label={t("monitoring.configuredDashboards")}
                control={
                    <Switch
                        inputProps={{role: "switch"}}
                        checked={list.mode === EMonitoringMode.CONFIGURED}
                        disabled={configure !== EMonitoringCapability.GRANTED || changingMode}
                        onChange={(_event, checked) => void changeMode(checked)}
                    />
                }
            />
            {modeError && <Alert severity="error">{t("monitoring.modeChangeError")}</Alert>}
        </Stack>
    )
    if (list.mode !== EMonitoringMode.CONFIGURED)
        return (
            <>
                {controls}
                {standard}
            </>
        )
    if (!list.dashboards.length) {
        return (
            <Stack spacing={2}>
                {controls}
                <Alert severity="info">{t("monitoring.noDashboards")}</Alert>
                {below}
                {footer}
            </Stack>
        )
    }
    return (
        <>
            {controls}
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
            {footer}
        </>
    )
}

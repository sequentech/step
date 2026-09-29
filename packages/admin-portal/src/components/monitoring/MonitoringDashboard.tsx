// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useState} from "react"
import {Alert, Box, Button, CircularProgress, Stack} from "@mui/material"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {MONITORING_GET_DASHBOARD} from "@/queries/MonitoringGetDashboard"
import {
    EMonitoringCapability,
    EMonitoringViewMode,
    EScopeSelector,
    MONITORING_DEFAULT_REFRESH_MS,
    type MonitoringDashboardSummary,
    type MonitoringGetDashboardQuery,
    type MonitoringGetDashboardVariables,
    type MonitoringScope,
    type MonitoringScopeOptions,
} from "./types"
import {layoutCells} from "./lib/layout"
import {parseDashboard, parseWidgets} from "./lib/parseDefinitions"
import {scopeLabel} from "./lib/scopeLabel"
import {useMonitoring} from "./MonitoringProvider"
import {usePageVisible} from "./usePageVisible"
import {MonitoringHeader} from "./MonitoringHeader"
import {MonitoringSelectors} from "./MonitoringSelectors"
import {MonitoringWidgetGrid} from "./MonitoringWidgetGrid"
import {MonitoringExportDialog} from "./MonitoringExportDialog"
import type {MonitoringWidgetContext} from "./MonitoringWidgetCard"

export interface MonitoringDashboardProps {
    electionEventId: string
    /** An election's page: the server pins the Post to it. */
    electionId?: string | null
    dashboards: MonitoringDashboardSummary[]
    configure: EMonitoringCapability
}

/** The scope a request carries: the dashboard's selectors only, with values the viewer may choose. */
function effectiveScope(
    chosen: MonitoringScope,
    selectors: EScopeSelector[],
    options: MonitoringScopeOptions,
    pinnedPost: string | null | undefined
): MonitoringScope {
    const scope: MonitoringScope = {}
    for (const selector of selectors) {
        const key = chosen[selector]
        if (!key || (selector === EScopeSelector.POST && pinnedPost)) continue
        if (!hasOption(options, selector, key)) continue
        scope[selector] = key
    }
    return scope
}

function hasOption(options: MonitoringScopeOptions, selector: EScopeSelector, key: string) {
    const list =
        selector === EScopeSelector.REGION
            ? options.regions
            : selector === EScopeSelector.POST
              ? options.posts
              : options.countries
    return list.some((option) => option.key === key)
}

export function MonitoringDashboard({
    electionEventId,
    electionId,
    dashboards,
    configure,
}: MonitoringDashboardProps) {
    const {t} = useTranslation()
    const {state, selectDashboard, setScope, actions} = useMonitoring()
    const [exporting, setExporting] = useState(false)
    const dashboardId = dashboards.some((dashboard) => dashboard.id === state.dashboardId)
        ? (state.dashboardId as string)
        : dashboards[0].id

    useEffect(() => {
        if (state.dashboardId !== dashboardId) selectDashboard(dashboardId)
    }, [state.dashboardId, dashboardId, selectDashboard])

    const {data, error, refetch, startPolling, stopPolling} = useQuery<
        MonitoringGetDashboardQuery,
        MonitoringGetDashboardVariables
    >(MONITORING_GET_DASHBOARD, {
        variables: {electionEventId, electionId: electionId ?? null, dashboardId},
    })

    // Every 30 s while the tab is shown and nobody is editing; charts are
    // drawn again only when the snapshot the poll reports is a new one.
    const visible = usePageVisible()
    const polling = visible && state.mode === EMonitoringViewMode.VIEW
    useEffect(() => {
        if (!polling) return
        startPolling(MONITORING_DEFAULT_REFRESH_MS)
        return () => stopPolling()
    }, [polling, startPolling, stopPolling])

    const response = data?.monitoringGetDashboard
    const parsed = useMemo(
        () =>
            response
                ? {
                      dashboard: parseDashboard(response.dashboard),
                      widgets: parseWidgets(response.widgets),
                  }
                : null,
        [response]
    )

    if (!response) {
        if (error) {
            return (
                <Alert
                    severity="error"
                    action={
                        <Button
                            color="inherit"
                            size="small"
                            onClick={() => void refetch().catch(() => undefined)}
                        >
                            {t("monitoring.retry")}
                        </Button>
                    }
                >
                    {t("monitoring.dashboardFailed")}
                </Alert>
            )
        }
        return (
            <Box sx={{display: "flex", justifyContent: "center", py: 4}}>
                <CircularProgress aria-label={t("monitoring.loading")} />
            </Box>
        )
    }
    if (!parsed?.dashboard.ok) {
        return (
            <Alert severity="error">
                {t("monitoring.dashboardInvalid", {
                    problem: parsed && !parsed.dashboard.ok ? parsed.dashboard.problem : "",
                })}
            </Alert>
        )
    }

    const dashboard = parsed.dashboard.value
    const selectors = dashboard.selectors ?? []
    const scope = effectiveScope(
        state.dashboardValues,
        selectors,
        response.scope_options,
        response.pinned_post
    )
    const label = scopeLabel({
        scope,
        selectors,
        options: response.scope_options,
        settings: response.settings,
        t,
        restricted: response.restricted,
        pinnedPost: response.pinned_post,
    })
    const cells = layoutCells(dashboard, parsed.widgets)
    const snapshot = response.snapshot ?? null
    const context: MonitoringWidgetContext = {
        electionEventId,
        electionId,
        dashboardId,
        scope,
        scopeLabel: label,
        snapshot,
        sources: response.sources ?? {},
        timeZone: response.settings?.time_zone ?? "UTC",
        configVersion: `${response.dashboard_revision}/${response.theme?.revision ?? ""}/${response.settings_revision}`,
        configure,
    }
    const onEditDashboard =
        configure === EMonitoringCapability.GRANTED && actions.onEditDashboard
            ? () => actions.onEditDashboard?.(dashboardId)
            : undefined

    return (
        <Stack spacing={2} sx={{py: 2}}>
            <MonitoringHeader
                dashboards={dashboards}
                dashboardId={dashboardId}
                onSelectDashboard={selectDashboard}
                widgetCount={cells.length}
                requirements={dashboard.requirements ?? []}
                snapshot={snapshot}
                timeZone={context.timeZone}
                onExport={snapshot ? () => setExporting(true) : undefined}
                onEditDashboard={onEditDashboard}
            />
            <MonitoringSelectors
                selectors={selectors}
                scope={scope}
                onChange={setScope}
                options={response.scope_options}
                settings={response.settings}
                restricted={response.restricted}
                pinnedPost={response.pinned_post}
            />
            <MonitoringWidgetGrid cells={cells} context={context} />
            {snapshot ? (
                <MonitoringExportDialog
                    open={exporting}
                    onClose={() => setExporting(false)}
                    title={dashboard.title}
                    scope={label}
                    timeZone={context.timeZone}
                    target={{
                        electionEventId,
                        electionId: electionId ?? null,
                        dashboardId,
                        widgetId: null,
                        scope,
                        selectorValues: {},
                        snapshotRevision: snapshot.revision,
                    }}
                />
            ) : null}
        </Stack>
    )
}

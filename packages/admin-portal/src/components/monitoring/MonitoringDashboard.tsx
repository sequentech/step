// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useEffect, useMemo, useRef, useState} from "react"
import {Alert, Box, Button, CircularProgress, Stack} from "@mui/material"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {MONITORING_GET_DASHBOARD} from "@/queries/MonitoringGetDashboard"
import {
    EMonitoringCapability,
    EMonitoringViewMode,
    EScopeSelector,
    type MonitoringDashboardSummary,
    type MonitoringGetDashboardQuery,
    type MonitoringGetDashboardVariables,
    type MonitoringScope,
    type MonitoringScopeOptions,
} from "./types"
import {layoutCells} from "./lib/layout"
import {
    BUSY_RETRY_MS,
    EMonitoringErrorCode,
    monitoringErrorCode,
    monitoringErrorMessage,
} from "./lib/errors"
import {dashboardSelectorValues} from "./lib/selectors"
import {parseDashboard, parseWidgets} from "./lib/parseDefinitions"
import {PINNED_BY_POST, scopeLabel} from "./lib/scopeLabel"
import {useMonitoring} from "./MonitoringProvider"
import {useMonitoringPolling} from "./useMonitoringPolling"
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
        if (!key || (pinnedPost && PINNED_BY_POST.has(selector))) continue
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
    const {state, selectDashboard, setScope, actions, widgetValues} = useMonitoring()
    const [exporting, setExporting] = useState(false)
    const dashboardId = dashboards.some((dashboard) => dashboard.id === state.dashboardId)
        ? (state.dashboardId as string)
        : dashboards[0].id

    useEffect(() => {
        if (state.dashboardId !== dashboardId) selectDashboard(dashboardId)
    }, [state.dashboardId, dashboardId, selectDashboard])

    const {data, error, refetch} = useQuery<
        MonitoringGetDashboardQuery,
        MonitoringGetDashboardVariables
    >(MONITORING_GET_DASHBOARD, {
        variables: {electionEventId, electionId: electionId ?? null, dashboardId},
    })
    const reload = useCallback(() => void refetch().catch(() => undefined), [refetch])

    // Every 30 s while the tab is shown and nobody is editing, and at once when
    // the tab is shown again. Charts are drawn again only when the snapshot the
    // poll reports is a new one; a widget that failed asks again on each poll.
    const [pollCount, setPollCount] = useState(0)
    useMonitoringPolling({
        active: state.mode === EMonitoringViewMode.VIEW,
        onPoll: () => {
            reload()
            setPollCount((count) => count + 1)
        },
    })
    // A poll that fails keeps this dashboard shown; the next one asks again.
    const shown = useRef<{dashboardId: string; data: MonitoringGetDashboardQuery} | null>(null)
    if (data) shown.current = {dashboardId, data}
    const kept =
        error && shown.current?.dashboardId === dashboardId ? shown.current.data : undefined

    const busy =
        !data && !kept && monitoringErrorCode(error) === EMonitoringErrorCode.BUSY
            ? error
            : undefined
    useEffect(() => {
        // Each busy answer waits, then asks again.
        if (!busy) return
        const timer = window.setTimeout(reload, BUSY_RETRY_MS)
        return () => window.clearTimeout(timer)
    }, [busy, reload])

    const response = (data ?? kept)?.monitoringGetDashboard
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
                        <Button color="inherit" size="small" onClick={reload}>
                            {t("monitoring.retry")}
                        </Button>
                    }
                >
                    {t("monitoring.dashboardFailed")}
                    {monitoringErrorCode(error) ? ` ${t(monitoringErrorMessage(error))}` : null}
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
    const eventDays = response.event_days ?? []
    const context: MonitoringWidgetContext = {
        electionEventId,
        electionId,
        dashboardId,
        scope,
        scopeLabel: label,
        snapshot,
        sources: response.sources ?? {},
        timeZone: response.settings?.time_zone ?? "UTC",
        eventDays,
        configVersion: `${response.dashboard_revision}/${response.theme?.revision ?? ""}/${response.settings_revision}`,
        pollCount,
        onSnapshotPruned: reload,
        configure,
    }
    const onEditDashboard =
        configure === EMonitoringCapability.GRANTED && actions.onEditDashboard
            ? () => actions.onEditDashboard?.(dashboardId)
            : undefined

    return (
        <Stack spacing={2} sx={{py: 2}}>
            <MonitoringHeader
                title={dashboard.title}
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
                    onSnapshotPruned={reload}
                    target={{
                        electionEventId,
                        electionId: electionId ?? null,
                        dashboardId,
                        widgetId: null,
                        scope,
                        selectorValues: {},
                        widgetSelectorValues: dashboardSelectorValues(
                            cells,
                            (cell) => widgetValues(cell),
                            {event_days: eventDays}
                        ),
                        snapshotRevision: snapshot.revision,
                    }}
                />
            ) : null}
        </Stack>
    )
}

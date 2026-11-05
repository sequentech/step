// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useRef, useState} from "react"
import {
    Box,
    Card,
    CardContent,
    Divider,
    LinearProgress,
    Skeleton,
    Stack,
    Typography,
    useTheme,
} from "@mui/material"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {MONITORING_RENDER_WIDGET} from "@/queries/MonitoringRenderWidget"
import {
    DEFAULT_WIDGET_HEIGHT,
    EColorScheme,
    EMonitoringCapability,
    EMonitoringExportFormat,
    EProducerState,
    EWidgetFailure,
    EWidgetRenderState,
    type MonitoringRenderWidgetQuery,
    type MonitoringRenderWidgetVariables,
    type MonitoringScope,
    type MonitoringSnapshot,
    type MonitoringSourceInfo,
} from "./types"
import type {LayoutCell} from "./lib/layout"
import {resolveSelectors, selectorValuesForRequest} from "./lib/selectors"
import {CHART_FONT_CSS} from "./lib/chartFonts"
import {
    BUSY_RETRY_MS,
    EMonitoringErrorCode,
    monitoringErrorCode,
    monitoringErrorMessage,
} from "./lib/errors"
import {useMonitoring} from "./MonitoringProvider"
import {useBucketedWidth} from "./useBucketedWidth"
import {MonitoringChartFrame} from "./MonitoringChartFrame"
import {MonitoringWidgetUnavailable} from "./MonitoringWidgetUnavailable"
import {MonitoringWidgetSelectors} from "./MonitoringWidgetSelectors"
import {MonitoringWidgetMenu} from "./MonitoringWidgetMenu"
import {MonitoringDataTableDialog} from "./MonitoringDataTableDialog"
import {widgetQueries} from "./lib/dataTables"
import {noticeText} from "./lib/notices"
import {MonitoringExportDialog} from "./MonitoringExportDialog"

/** What every widget of a dashboard shares. */
export interface MonitoringWidgetContext {
    electionEventId: string
    electionId?: string | null
    dashboardId: string
    scope: MonitoringScope
    scopeLabel: string
    snapshot: MonitoringSnapshot | null
    sources: Record<string, MonitoringSourceInfo>
    timeZone: string
    /** The options of a selector with `options_from: event_days`. */
    eventDays: string[]
    /** Changes when the dashboard, theme or settings are saved, so charts are drawn again. */
    configVersion: string
    /** Counts the dashboard's polls: a widget that failed asks again on each. */
    pollCount: number
    /** The update being exported is gone: the dashboard is asked for the current one. */
    onSnapshotPruned?: () => void
    configure: EMonitoringCapability
}

export interface MonitoringWidgetCardProps {
    cell: LayoutCell
    context: MonitoringWidgetContext
}

enum EWidgetDialog {
    NONE = "NONE",
    DATA = "DATA",
    EXPORT = "EXPORT",
}

export function MonitoringWidgetCard({cell, context}: MonitoringWidgetCardProps) {
    const {t, i18n} = useTranslation()
    const theme = useTheme()
    const {widgetValues, setWidgetValue, actions} = useMonitoring()
    const [dialog, setDialog] = useState(EWidgetDialog.NONE)
    const body = useRef<HTMLDivElement>(null)
    const width = useBucketedWidth(body)
    const widget = cell.widget
    const picks = widgetValues(cell)
    const eventDays = context.eventDays

    const selectors = useMemo(
        () => (widget ? resolveSelectors(widget, cell.values, picks, {event_days: eventDays}) : []),
        [widget, cell.values, picks, eventDays]
    )
    const selectorValues = useMemo(() => selectorValuesForRequest(selectors), [selectors])
    const source = widget ? context.sources[widget.source] : undefined
    const notConnected = source?.producer === EProducerState.NOT_CONNECTED
    const colorScheme = theme.palette.mode === "dark" ? EColorScheme.DARK : EColorScheme.LIGHT

    const {data, previousData, loading, error, refetch} = useQuery<
        MonitoringRenderWidgetQuery,
        MonitoringRenderWidgetVariables
    >(MONITORING_RENDER_WIDGET, {
        variables: {
            electionEventId: context.electionEventId,
            electionId: context.electionId ?? null,
            dashboardId: context.dashboardId,
            widgetId: cell.widgetId,
            scope: context.scope,
            selectorValues,
            snapshotRevision: context.snapshot?.revision ?? null,
            width: width ?? 0,
            colorScheme,
            locale: i18n.language,
        },
        // Polls do not draw a widget again unless the snapshot, scope or
        // selectors changed, since the request stays the same. Renders are not
        // cached: each snapshot revision would leave its SVGs in the cache.
        fetchPolicy: "no-cache",
        skip: !widget || notConnected || width === null,
    })

    // A saved change to the widget, dashboard, theme or settings keeps the
    // request the same, so it is asked again explicitly.
    const version = `${cell.revision ?? ""}/${context.configVersion}`
    const drawnVersion = useRef(version)
    useEffect(() => {
        if (drawnVersion.current === version) return
        drawnVersion.current = version
        void refetch().catch(() => undefined)
    }, [version, refetch])

    // A failed widget asks again on the dashboard's next poll; a busy one sooner.
    const errorCode = error ? monitoringErrorCode(error) : undefined
    const polled = useRef(context.pollCount)
    useEffect(() => {
        if (polled.current === context.pollCount) return
        polled.current = context.pollCount
        if (error) void refetch().catch(() => undefined)
    }, [context.pollCount, error, refetch])
    const busy = errorCode === EMonitoringErrorCode.BUSY ? error : undefined
    useEffect(() => {
        // Each busy answer waits, then asks again.
        if (!busy) return
        const timer = window.setTimeout(() => void refetch().catch(() => undefined), BUSY_RETRY_MS)
        return () => window.clearTimeout(timer)
    }, [busy, refetch])

    // The previous chart only while the next one is on its way, marked as such;
    // after a failure, the failure.
    const updating = loading && !data && Boolean(previousData)
    const render = error
        ? undefined
        : (data ?? (loading ? previousData : undefined))?.monitoringRenderWidget
    const hasData = Boolean(render?.tables?.some((query) => query.table) || render?.table)
    const title = widget?.title ?? cell.widgetId
    const height = widget?.height ?? DEFAULT_WIDGET_HEIGHT
    const canConfigure = context.configure === EMonitoringCapability.GRANTED

    const content = () => {
        if (!widget) {
            return (
                <MonitoringWidgetUnavailable
                    state={EWidgetRenderState.INVALID}
                    problem={
                        cell.problem === "missing"
                            ? t("monitoring.widget.missing", {id: cell.widgetId})
                            : cell.problem
                    }
                    title={title}
                />
            )
        }
        if (notConnected) {
            return (
                <MonitoringWidgetUnavailable
                    state={EWidgetRenderState.NOT_CONNECTED}
                    reason={source?.reason}
                    title={title}
                />
            )
        }
        if (error) {
            return (
                <MonitoringWidgetUnavailable
                    state={EWidgetFailure.REQUEST_FAILED}
                    problem={errorCode ? t(monitoringErrorMessage(error)) : undefined}
                    title={title}
                />
            )
        }
        if (!render) {
            return (
                <Skeleton
                    variant="rectangular"
                    height={height}
                    role="progressbar"
                    aria-label={t("monitoring.widget.loading", {widget: title})}
                />
            )
        }
        if (render.state === EWidgetRenderState.RENDERED && render.svg) {
            return (
                <MonitoringChartFrame
                    svg={render.svg}
                    title={t("monitoring.frame.title", {widget: title})}
                    height={height}
                    colorScheme={colorScheme}
                    fontCss={CHART_FONT_CSS}
                />
            )
        }
        return (
            <MonitoringWidgetUnavailable
                state={
                    render.state === EWidgetRenderState.RENDERED
                        ? EWidgetRenderState.RENDER_FAILED
                        : render.state
                }
                reason={render.reason}
                diagnostics={render.diagnostics}
                table={render.table}
                title={title}
            />
        )
    }

    return (
        <Card variant="outlined" sx={{height: "100%"}} data-widget-id={cell.widgetId}>
            <CardContent>
                <Stack spacing={1}>
                    <Stack direction="row" alignItems="center" spacing={1}>
                        <Typography
                            variant="subtitle1"
                            component="h3"
                            sx={{flexGrow: 1, minWidth: 0, fontWeight: 600, color: "brandColor"}}
                        >
                            {title}
                        </Typography>
                        <MonitoringWidgetMenu
                            widgetTitle={title}
                            onConfigure={
                                canConfigure && actions.onConfigureWidget
                                    ? () =>
                                          actions.onConfigureWidget?.(
                                              cell.widgetId,
                                              width ?? undefined
                                          )
                                    : undefined
                            }
                            onViewData={hasData ? () => setDialog(EWidgetDialog.DATA) : undefined}
                            onExport={
                                widget && context.snapshot
                                    ? () => setDialog(EWidgetDialog.EXPORT)
                                    : undefined
                            }
                            onDuplicate={
                                canConfigure && actions.onDuplicateWidget
                                    ? () => actions.onDuplicateWidget?.(cell.widgetId)
                                    : undefined
                            }
                        />
                    </Stack>
                    <Divider />
                    {widget?.description ? (
                        <Typography variant="caption" color="text.secondary">
                            {widget.description}
                        </Typography>
                    ) : null}
                    <MonitoringWidgetSelectors
                        widgetId={cell.key}
                        selectors={selectors}
                        onChange={(name, value) => setWidgetValue(cell, name, value)}
                    />
                    <Box ref={body} aria-busy={updating || undefined}>
                        {updating ? (
                            <Stack spacing={0.5} role="status" sx={{mb: 1}}>
                                <LinearProgress
                                    aria-label={t("monitoring.widget.updating", {widget: title})}
                                />
                                <Typography variant="caption" color="text.secondary">
                                    {t("monitoring.widget.updatingNote")}
                                </Typography>
                            </Stack>
                        ) : null}
                        <Box sx={updating ? {opacity: 0.5} : undefined}>{content()}</Box>
                    </Box>
                    {render?.notices.map((notice) => (
                        <Typography key={notice} variant="caption" color="text.secondary">
                            {noticeText(t, notice)}
                        </Typography>
                    ))}
                </Stack>
            </CardContent>
            {render && hasData ? (
                <MonitoringDataTableDialog
                    open={dialog === EWidgetDialog.DATA}
                    onClose={() => setDialog(EWidgetDialog.NONE)}
                    title={title}
                    scope={context.scopeLabel}
                    table={render.table}
                    tables={render.tables}
                    queries={widgetQueries(widget)}
                    notices={render.notices}
                />
            ) : null}
            {context.snapshot ? (
                <MonitoringExportDialog
                    open={dialog === EWidgetDialog.EXPORT}
                    onClose={() => setDialog(EWidgetDialog.NONE)}
                    title={title}
                    scope={context.scopeLabel}
                    timeZone={context.timeZone}
                    initialFormat={EMonitoringExportFormat.CSV}
                    onSnapshotPruned={context.onSnapshotPruned}
                    widgets={widget ? [widget] : []}
                    target={{
                        electionEventId: context.electionEventId,
                        electionId: context.electionId ?? null,
                        dashboardId: context.dashboardId,
                        widgetId: cell.widgetId,
                        scope: context.scope,
                        selectorValues,
                        snapshotRevision: context.snapshot.revision,
                    }}
                />
            ) : null}
        </Card>
    )
}

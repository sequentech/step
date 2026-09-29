// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic monitoring responses in the shapes of the Harvest routes, and
// GraphQL boundary handlers that answer the monitoring operations with them.
// Every value is invented.
import type {FetchResult, Operation} from "@apollo/client"
import {FIXED_TIME, STORY_IDS} from "@/__stories__/fixtures"
import {pending} from "../../../../../ui-essentials/.storybook/screens"
import {
    EColumnKind,
    EDataSource,
    EMonitoringMode,
    EProducerState,
    ESelectorControl,
    EScopeSelector,
    EWidgetRenderState,
    type MonitoringDashboard,
    type MonitoringGetDashboardResponse,
    type MonitoringListDashboardsResponse,
    type MonitoringRenderWidgetResponse,
    type MonitoringRenderWidgetVariables,
    type MonitoringSourceInfo,
    type MonitoringTable,
    type MonitoringWidget,
} from "../types"

export const MONITORING_SNAPSHOT = {revision: 41, as_of: FIXED_TIME, checked_at: FIXED_TIME}

export const POSTS = {
    madrid: STORY_IDS.election,
    paris: STORY_IDS.secondElection,
}

export const turnoutSummary: MonitoringWidget = {
    id: "turnout-summary",
    title: "Voter turnout",
    source: EDataSource.VOTER_TURNOUT,
    requirements: ["SW-F-0259"],
    query: {template: "summary", measures: ["registered", "pre_enrolled", "voted"]},
    chart: {charts: {kpi: {type: "kpi", query: "data"}}, rows: ["kpi"]},
    height: 160,
}

export const turnoutByGroup: MonitoringWidget = {
    id: "turnout-by-group",
    title: "Turnout by group",
    source: EDataSource.VOTER_TURNOUT,
    requirements: ["SW-F-0260", "SW-F-0372"],
    selectors: {
        breakdown: {
            label: "Breakdown",
            options: {sex: "Sex", age_band: "Age", status: "Status abroad"},
            default: "age_band",
        },
        measure: {
            label: "Show",
            options: {
                voted_reg: "Voted of registered",
                voted_pre: "Voted of pre-enrolled",
                pre_reg: "Pre-enrolled of registered",
            },
            default: "voted_reg",
            maps: {
                voted_reg: ["voted", "registered"],
                voted_pre: ["voted", "pre_enrolled"],
                pre_reg: ["pre_enrolled", "registered"],
            },
        },
    },
    query: {template: "by_group", group_by: {selector: "breakdown"}, ratio: {selector: "measure"}},
    chart: {charts: {bars: {type: "bar", query: "data", x: "group", y: "pct"}}, rows: ["bars"]},
}

export const votingActivity: MonitoringWidget = {
    id: "voting-activity",
    title: "Voting activity",
    source: EDataSource.VOTING_ENROLLMENT_ACTIVITY,
    requirements: ["SW-F-0267"],
    selectors: {
        grain: {
            label: "Grain",
            options: {hour: "Hourly", day: "Daily"},
            default: "day",
            control: ESelectorControl.TOGGLE,
        },
    },
    query: {template: "timeseries", grain: {selector: "grain"}, measures: ["voted"]},
    chart: {
        charts: {area: {type: "area", query: "data", x: "bucket_start", y: "voted"}},
        rows: ["area"],
    },
}

export const attackDetections: MonitoringWidget = {
    id: "attack-detections",
    title: "Attack detections",
    source: EDataSource.ATTACK_DETECTIONS,
    requirements: ["SW-F-0283"],
    query: {template: "summary", measures: ["detections"]},
    chart: {charts: {kpi: {type: "kpi", query: "data"}}, rows: ["kpi"]},
    height: 160,
}

export const pollStatus: MonitoringWidget = {
    id: "poll-status",
    title: "Poll status",
    source: EDataSource.POLL_STATUS,
    requirements: ["SW-F-0256"],
    query: {template: "by_measure", measures: ["initialized", "opened", "closed"]},
    chart: {charts: {bars: {type: "bar", query: "data", x: "measure", y: "value"}}, rows: ["bars"]},
}

export const WIDGETS: MonitoringWidget[] = [
    turnoutSummary,
    turnoutByGroup,
    votingActivity,
    pollStatus,
    attackDetections,
]

export const overviewDashboard: MonitoringDashboard = {
    id: "overview",
    title: "Monitoring overview",
    requirements: ["SW-F-0247", "SW-F-0279", "SW-F-0365"],
    order: 0,
    selectors: [EScopeSelector.REGION, EScopeSelector.POST, EScopeSelector.COUNTRY],
    theme: "comelec",
    layout: [
        {widget: "turnout-summary", width: 12},
        {widget: "turnout-by-group", width: 6, values: {measure: "voted_pre"}},
        {widget: "voting-activity", width: 6},
        {widget: "poll-status", width: 6},
        {widget: "attack-detections", width: 6},
    ],
}

export const turnoutDashboard: MonitoringDashboard = {
    id: "req-0260",
    title: "Voted vs pre-enrolled",
    requirements: ["SW-F-0260", "SW-F-0372"],
    order: 1,
    selectors: [EScopeSelector.REGION, EScopeSelector.POST, EScopeSelector.COUNTRY],
    layout: [{widget: "turnout-by-group", width: 12, values: {measure: "voted_pre"}}],
}

export const DASHBOARDS = [overviewDashboard, turnoutDashboard]

export function listDashboardsResponse(
    mode: EMonitoringMode = EMonitoringMode.CONFIGURED
): MonitoringListDashboardsResponse {
    return {
        mode,
        dashboards: DASHBOARDS.map((dashboard) => ({
            id: dashboard.id,
            title: dashboard.title,
            requirements: dashboard.requirements ?? [],
            widget_count: dashboard.layout.length,
        })),
        snapshot: MONITORING_SNAPSHOT,
    }
}

const connected = (dimensions: string[]): MonitoringSourceInfo => ({
    counting_unit: "DISTINCT_VOTERS",
    measures: ["registered", "pre_enrolled", "voted"],
    templates: ["summary", "by_group", "by_post", "timeseries"],
    dimensions,
    producer: EProducerState.CONNECTED,
    reason: null,
})

export interface DashboardOptions {
    dashboard?: MonitoringDashboard
    restricted?: boolean
    pinnedPost?: string | null
    snapshot?: typeof MONITORING_SNAPSHOT | null
    /** Replaces the widget definitions, e.g. with one that is not valid. */
    widgets?: Record<string, unknown>
}

export function getDashboardResponse({
    dashboard = overviewDashboard,
    restricted = false,
    pinnedPost = null,
    snapshot = MONITORING_SNAPSHOT,
    widgets,
}: DashboardOptions = {}): MonitoringGetDashboardResponse {
    const definitions = widgets ?? Object.fromEntries(WIDGETS.map((widget) => [widget.id, widget]))
    return {
        dashboard,
        dashboard_revision: 3,
        widgets: Object.fromEntries(
            Object.entries(definitions).map(([id, definition]) => [id, {definition, revision: 2}])
        ),
        theme: {id: "comelec", revision: 1},
        settings: {time_zone: "Asia/Manila", unknown_label: "Unknown", selectors: {}},
        settings_revision: 1,
        scope_options: {
            regions: [
                {key: "north", label: "North"},
                {key: "south", label: "South"},
            ],
            posts: restricted
                ? [{key: POSTS.madrid, label: "Madrid", region: "north"}]
                : [
                      {key: POSTS.madrid, label: "Madrid", region: "north"},
                      {key: POSTS.paris, label: "Paris", region: "south"},
                  ],
            countries: [
                {key: "ES", label: "Spain"},
                {key: "FR", label: "France"},
            ],
        },
        restricted,
        pinned_post: pinnedPost,
        sources: {
            [EDataSource.VOTER_TURNOUT]: connected([
                "region",
                "post",
                "country",
                "sex",
                "age_band",
            ]),
            [EDataSource.VOTING_ENROLLMENT_ACTIVITY]: connected(["region", "post", "country"]),
            [EDataSource.POLL_STATUS]: {
                ...connected(["region", "post"]),
                counting_unit: "POSTS_IN_SCOPE",
            },
            [EDataSource.ATTACK_DETECTIONS]: {
                ...connected([]),
                counting_unit: "DETECTIONS",
                producer: EProducerState.NOT_CONNECTED,
                reason: "ATTACK_DETECTION_FEED",
            },
        },
        snapshot,
    }
}

/** A bar chart as the renderer would send it, with one bar per value. */
export function barChartSvg(values: number[], color = "#2c6fbb"): string {
    const bars = values
        .map(
            (value, index) =>
                `<rect x="${10 + index * 60}" y="${150 - value}" width="40" height="${value}" fill="${color}"/>` +
                `<text x="${30 + index * 60}" y="170" font-size="12" text-anchor="middle">${value}</text>`
        )
        .join("")
    return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${values.length * 60 + 20} 180" width="100%">${bars}</svg>`
}

export const turnoutTable: MonitoringTable = {
    columns: [
        {name: "group", kind: EColumnKind.TEXT},
        {name: "voted", kind: EColumnKind.INTEGER},
        {name: "pre_enrolled", kind: EColumnKind.INTEGER},
        {name: "pct", kind: EColumnKind.NUMBER},
        {name: "position", kind: EColumnKind.INTEGER},
    ],
    rows: [
        ["18–29", 120432, 226000, 0.5329, 1],
        ["30–59", 402113, 610220, 0.659, 2],
        ["Unknown", 1204, 0, null, 3],
    ],
}

export const summaryTable: MonitoringTable = {
    columns: [
        {name: "registered", kind: EColumnKind.INTEGER},
        {name: "voted", kind: EColumnKind.INTEGER},
        {name: "pct", kind: EColumnKind.NUMBER},
    ],
    rows: [[874624, 465321, 0.532]],
}

export function renderResponse(
    overrides: Partial<MonitoringRenderWidgetResponse> = {}
): MonitoringRenderWidgetResponse {
    return {
        state: EWidgetRenderState.RENDERED,
        reason: null,
        svg: barChartSvg([120, 90, 40]),
        table: turnoutTable,
        notices: [],
        diagnostics: [],
        ignored_selectors: [],
        render_ms: 42,
        snapshot_revision: MONITORING_SNAPSHOT.revision,
        as_of: FIXED_TIME,
        ...overrides,
    }
}

export enum EMonitoringListScenario {
    CONFIGURED = "CONFIGURED",
    LEGACY = "LEGACY",
    ERROR = "ERROR",
    LOADING = "LOADING",
}

export interface MonitoringHandlerOptions extends DashboardOptions {
    list?: EMonitoringListScenario
    /** A widget's render, by widget id; the others draw a bar chart of their own. */
    renders?: Record<string, Partial<MonitoringRenderWidgetResponse>>
}

/** GraphQL boundary handlers for the monitoring operations a dashboard makes. */
export function monitoringHandlers(options: MonitoringHandlerOptions = {}) {
    const {list = EMonitoringListScenario.CONFIGURED, renders = {}} = options
    return {
        MonitoringListDashboards: (): FetchResult | Promise<FetchResult> => {
            if (list === EMonitoringListScenario.LOADING) return pending()
            if (list === EMonitoringListScenario.ERROR)
                throw new Error("Synthetic monitoring outage")
            return {
                data: {
                    monitoringListDashboards: listDashboardsResponse(
                        list === EMonitoringListScenario.LEGACY
                            ? EMonitoringMode.LEGACY
                            : EMonitoringMode.CONFIGURED
                    ),
                },
            }
        },
        MonitoringGetDashboard: (operation: Operation): FetchResult => {
            const dashboard =
                DASHBOARDS.find((candidate) => candidate.id === operation.variables.dashboardId) ??
                overviewDashboard
            return {data: {monitoringGetDashboard: getDashboardResponse({...options, dashboard})}}
        },
        MonitoringRenderWidget: (operation: Operation): FetchResult => {
            const variables = operation.variables as MonitoringRenderWidgetVariables
            const override = renders[variables.widgetId] ?? {}
            const widgetIndex = WIDGETS.findIndex((widget) => widget.id === variables.widgetId)
            return {
                data: {
                    monitoringRenderWidget: renderResponse({
                        svg: barChartSvg([40 + widgetIndex * 20, 90, 120 - widgetIndex * 10]),
                        table:
                            variables.widgetId === turnoutSummary.id ? summaryTable : turnoutTable,
                        ...override,
                    }),
                },
            }
        },
        MonitoringExport: (): FetchResult => ({
            data: {
                monitoringExport: {
                    document_id: "monitoring-export-document",
                    task_execution: {
                        id: "monitoring-export-task",
                        name: "EXPORT_MONITORING_DATA",
                        execution_status: "IN_PROGRESS",
                    },
                },
            },
        }),
    }
}

/**
 * The monitoring answer of an event that keeps the standard dashboard, for the
 * stories of screens that include the Dashboard tab.
 */
export function legacyMonitoring() {
    return {
        MonitoringListDashboards: (): FetchResult => ({
            data: {monitoringListDashboards: listDashboardsResponse(EMonitoringMode.LEGACY)},
        }),
    }
}

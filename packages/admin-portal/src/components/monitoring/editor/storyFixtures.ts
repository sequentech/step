// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** Synthetic monitoring documents and replies for the editor's stories. */

import {fn} from "storybook/test"
import type {IMonitoringEditorApi} from "./api"
import {
    EMonitoringConfigKind,
    EMonitoringProblemSeverity,
    EMonitoringRenderState,
    EMonitoringSaveStatus,
    EMonitoringValidationResult,
    type IMonitoringDashboardDefinition,
    type IMonitoringProblem,
    type IMonitoringRenderResponse,
    type IMonitoringSourceInfo,
    type TMonitoringSaveOutcome,
} from "./types"

export const WIDGET_YAML = `# One turnout ratio broken down by a voter dimension.
id: turnout-by-group
title: Turnout by group
source: voter_turnout
requirements: [SW-F-0259, SW-F-0260]
follows: [region, post, country]
selectors:
  breakdown:
    label: Breakdown
    options: {sex: Sex, age_band: Age, status: Status abroad}
    default: age_band
  measure:
    label: Show
    options: {voted_reg: Voted of registered, voted_pre: Voted of pre-enrolled}
    default: voted_reg
    maps: {voted_reg: [voted, registered], voted_pre: [voted, pre_enrolled]}
query:
  template: by_group
  group_by: {selector: breakdown}
  ratio: {selector: measure}
height: 240
chart:
  charts:
    bars:
      type: bar
      query: data
      x: group
      y: pct
  rows: [bars]
`

export const SUMMARY_YAML = `id: turnout-summary
title: Voter turnout
source: voter_turnout
requirements: [SW-F-0259]
query:
  template: summary
  measures: [registered, pre_enrolled, voted]
chart:
  charts:
    kpi: {type: kpi, query: data, value: voted}
  rows: [kpi]
`

export const THEME_YAML = `id: default
title: COMELEC
base: clarity
style:
  palette: ["#0f054c", "#2c7be5", "#43e3a1"]
`

export const DASHBOARD: IMonitoringDashboardDefinition = {
    id: "req-0260",
    title: "Voted vs pre-enrolled",
    requirements: ["SW-F-0260", "SW-F-0372"],
    selectors: ["region", "post", "country"],
    theme: "default",
    layout: [
        {widget: "turnout-summary", width: 12},
        {widget: "turnout-by-group", width: 6, values: {measure: "voted_pre"}},
        {widget: "voting-activity", width: 6},
    ],
}

export const DASHBOARD_YAML = `# The SW-F-0260 dashboard.
id: req-0260
title: Voted vs pre-enrolled
requirements: [SW-F-0260, SW-F-0372]
selectors: [region, post, country]
theme: default
layout:
  - {widget: turnout-summary, width: 12}
  - {widget: turnout-by-group, width: 6, values: {measure: voted_pre}}
  - {widget: voting-activity, width: 6}
`

export const SOURCES: Record<string, IMonitoringSourceInfo> = {
    voter_turnout: {
        counting_unit: "DISTINCT_VOTERS",
        measures: ["registered", "pre_enrolled", "voted"],
        templates: ["summary", "by_group", "by_post", "timeseries"],
        dimensions: ["sex", "age_band", "status", "region", "country"],
        producer: "CONNECTED",
    },
    attack_detections: {
        counting_unit: "DETECTIONS",
        measures: ["detections"],
        templates: ["summary", "timeseries"],
        dimensions: [],
        producer: "NOT_CONNECTED",
        reason: "VOTE-SECOPS does not report detections yet.",
    },
}

/** A small bar chart, as the renderer would answer. */
export const CHART_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="360" height="120" viewBox="0 0 360 120" role="img"><title>Turnout by group</title><rect x="10" y="10" width="220" height="24" fill="#2c7be5"/><rect x="10" y="46" width="160" height="24" fill="#2c7be5"/><rect x="10" y="82" width="90" height="24" fill="#2c7be5"/><text x="240" y="28" font-size="12">61.2%</text><text x="180" y="64" font-size="12">44.5%</text><text x="110" y="100" font-size="12">25.0%</text></svg>`

export const RENDERED: IMonitoringRenderResponse = {
    state: EMonitoringRenderState.RENDERED,
    svg: CHART_SVG,
    table: {
        columns: [
            {name: "group", kind: "text"},
            {name: "numerator", kind: "integer"},
            {name: "denominator", kind: "integer"},
            {name: "pct", kind: "ratio"},
        ],
        rows: [
            ["18–29", 612, 1000, 0.612],
            ["30–59", 445, 1000, 0.445],
            ["Unknown", 25, 100, 0.25],
        ],
    },
    notices: [],
    diagnostics: [],
    ignored_selectors: [],
    render_ms: 42,
    snapshot_revision: 17,
    as_of: "2026-09-29T10:00:00Z",
}

export const CHART_WARNING: IMonitoringProblem = {
    severity: EMonitoringProblemSeverity.WARNING,
    code: "chart_schema",
    path: "chart.charts.bars",
    message: "Bars sorted by value hide the Unknown group's position.",
    engine_code: "WARN-SORT-001",
}

export const FORBIDDEN_KEY: IMonitoringProblem = {
    severity: EMonitoringProblemSeverity.ERROR,
    code: "forbidden_key",
    path: "query.sql",
    message: "A query only chooses a template and its parameters; `sql` is not one.",
}

/** An editor API whose every call answers at once, recorded for assertions. */
export const fakeEditorApi = (
    overrides: Partial<IMonitoringEditorApi> = {}
): IMonitoringEditorApi => ({
    validateConfig: fn(async () => ({
        result: EMonitoringValidationResult.VALID,
        problems: [],
        preview: RENDERED,
    })),
    saveConfig: fn(
        async (): Promise<TMonitoringSaveOutcome> => ({
            status: EMonitoringSaveStatus.SAVED,
            revision: 8,
            generation: 3,
        })
    ),
    renderWidget: fn(async () => RENDERED),
    getConfig: fn(async ({kind, key}) => ({
        kind,
        key,
        yaml:
            kind === EMonitoringConfigKind.THEME
                ? THEME_YAML
                : kind === EMonitoringConfigKind.DASHBOARD
                  ? DASHBOARD_YAML
                  : WIDGET_YAML,
        revision: 7,
        origin: "EDITOR",
        author: {id: "u-ana", name: "Ana Reyes"},
        created_at: "2026-09-28T08:30:00Z",
    })),
    listConfig: fn(async () => []),
    listPresets: fn(async () => [
        {id: "comelec", version: 3, title: "COMELEC"},
        {id: "campus", version: 1, title: "Campus elections"},
    ]),
    resetToPreset: fn(async () => ({generation: 4})),
    ...overrides,
})

const ACTIVITY_YAML = `id: voting-activity
title: Voting activity
source: voting_enrollment_activity
requirements: [SW-F-0372]
query: {template: timeseries, measures: [voted]}
chart: {charts: {line: {type: line, query: data, x: day, y: voted}}, rows: [line]}
`

const ATTACKS_YAML = `id: attack-log
title: Attack detections
source: attack_detections
requirements: [SW-F-0301]
query: {template: summary, measures: [detections]}
chart: {charts: {kpi: {type: kpi, query: data, value: detections}}, rows: [kpi]}
`

/** The event's documents, by kind and key, as an editor story's `getConfig` answers. */
export const EVENT_DOCUMENTS: Record<string, string> = {
    "dashboard/req-0260": DASHBOARD_YAML,
    "widget/turnout-summary": SUMMARY_YAML,
    "widget/turnout-by-group": WIDGET_YAML,
    "widget/voting-activity": ACTIVITY_YAML,
    "widget/attack-log": ATTACKS_YAML,
    "theme/default": THEME_YAML,
    "theme/dark": THEME_YAML.replace("id: default", "id: dark"),
}

/** An editor API over {@link EVENT_DOCUMENTS}: a whole event's configuration. */
export const eventEditorApi = (overrides: Partial<IMonitoringEditorApi> = {}) =>
    fakeEditorApi({
        listConfig: fn(async () =>
            Object.keys(EVENT_DOCUMENTS).map((path) => {
                const [kind, key] = path.split("/")
                return {kind: kind as EMonitoringConfigKind, key, revision: 7}
            })
        ),
        getConfig: fn(async ({kind, key}) => {
            const yaml = EVENT_DOCUMENTS[`${kind}/${key}`]
            if (yaml === undefined) throw new Error(`${kind} ${key} not found`)
            return {
                kind,
                key,
                yaml,
                revision: 7,
                origin: "EDITOR",
                author: {id: "u-ana", name: "Ana Reyes"},
                created_at: "2026-09-28T08:30:00Z",
            }
        }),
        ...overrides,
    })

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What the monitoring editor exchanges with Harvest's `/monitoring/*` routes,
 * through their Hasura actions. Only what the editor needs: the view's own
 * types live in `../types.ts`.
 */

/** The kinds of configuration document an event stores. */
export enum EMonitoringConfigKind {
    DASHBOARD = "dashboard",
    WIDGET = "widget",
    THEME = "theme",
    SETTINGS = "settings",
}

export enum EMonitoringProblemSeverity {
    ERROR = "ERROR",
    WARNING = "WARNING",
}

/** One thing wrong with a document, as sequent-core's policy reports it. */
export interface IMonitoringProblem {
    severity: EMonitoringProblemSeverity
    /** Stable identifier (`forbidden_key`, `chart_schema`, …); safe to match on. */
    code: string
    /** Dotted path into the document, `chart.charts.bars[0].query`; empty for the whole document. */
    path: string
    message: string
    /** The dbt Charts code (`ERR-…`, `WARN-…`) behind a chart problem. */
    engine_code?: string | null
}

export enum EMonitoringRenderState {
    RENDERED = "RENDERED",
    NOT_CONNECTED = "NOT_CONNECTED",
    NO_SNAPSHOT = "NO_SNAPSHOT",
    SCOPE_PENDING = "SCOPE_PENDING",
    RENDER_FAILED = "RENDER_FAILED",
    INVALID = "INVALID",
}

export enum EMonitoringColorScheme {
    LIGHT = "LIGHT",
    DARK = "DARK",
}

export interface IMonitoringScope {
    region?: string | null
    post?: string | null
    country?: string | null
}

export interface IMonitoringTableColumn {
    name: string
    kind: string
}

export interface IMonitoringTable {
    columns: IMonitoringTableColumn[]
    rows: unknown[][]
}

export interface IMonitoringRenderResponse {
    state: EMonitoringRenderState
    reason?: string | null
    svg?: string | null
    table?: IMonitoringTable | null
    notices?: string[]
    diagnostics?: IMonitoringProblem[]
    ignored_selectors?: string[]
    render_ms?: number | null
    snapshot_revision?: number | null
    as_of?: string | null
}

export interface IMonitoringDraft {
    widget_yaml?: string
    theme_yaml?: string
}

export interface IMonitoringRenderRequest {
    dashboard_id: string
    widget_id: string
    election_id?: string | null
    scope: IMonitoringScope
    selector_values: Record<string, string>
    width: number
    color_scheme: EMonitoringColorScheme
    locale: string
    draft?: IMonitoringDraft
}

export enum EMonitoringValidationResult {
    VALID = "VALID",
    INVALID = "INVALID",
}

export interface IMonitoringValidateResponse {
    result: EMonitoringValidationResult
    problems: IMonitoringProblem[]
    preview?: IMonitoringRenderResponse | null
}

export enum EMonitoringSaveChange {
    UPSERT = "UPSERT",
    DELETE = "DELETE",
}

export interface IMonitoringSaveRequest {
    kind: EMonitoringConfigKind
    key: string
    yaml?: string
    expected_revision?: number | null
    change: EMonitoringSaveChange
}

export interface IMonitoringAuthor {
    id: string
    name?: string | null
}

/** What a save came to: stored, refused because someone saved first, or refused as invalid. */
export enum EMonitoringSaveStatus {
    SAVED = "SAVED",
    CONFLICT = "CONFLICT",
    INVALID = "INVALID",
}

export type TMonitoringSaveOutcome =
    | {status: EMonitoringSaveStatus.SAVED; revision: number; generation: number}
    | {
          status: EMonitoringSaveStatus.CONFLICT
          current_revision: number
          author?: IMonitoringAuthor | null
          time?: string | null
      }
    | {status: EMonitoringSaveStatus.INVALID; problems: IMonitoringProblem[]}

export interface IMonitoringConfigDocument {
    kind: EMonitoringConfigKind
    key: string
    yaml: string
    revision: number
    origin?: string | null
    author?: IMonitoringAuthor | null
    created_at?: string | null
}

export interface IMonitoringConfigListEntry {
    kind: EMonitoringConfigKind
    key: string
    revision: number
    origin?: string | null
    author?: IMonitoringAuthor | null
    created_at?: string | null
}

export interface IMonitoringPreset {
    id: string
    version: string | number
    title: string
}

/** A data source as `get-dashboard` describes it: what a query may ask of it. */
export interface IMonitoringSourceInfo {
    counting_unit: string
    measures: string[]
    templates: string[]
    dimensions: string[]
    producer?: string | null
    reason?: string | null
}

/** The widget definition as YAML reads, only as far as the forms go. */
export interface IMonitoringSelectorDefinition {
    label?: string
    options?: Record<string, string>
    options_from?: string
    default?: string
    control?: string
    when?: {selector: string; in: string[]}
    maps?: Record<string, unknown>
}

export interface IMonitoringQueryDefinition {
    template?: string
    measures?: unknown
    ratio?: unknown
    group_by?: unknown
    filters?: Record<string, unknown>
    grain?: unknown
    day?: unknown
    sort?: unknown
    limit?: unknown
    labels?: Record<string, string>
}

export interface IMonitoringWidgetDefinition {
    id?: string
    title?: string
    source?: string
    requirements?: string[]
    follows?: string[]
    selectors?: Record<string, IMonitoringSelectorDefinition>
    query?: IMonitoringQueryDefinition
    queries?: Record<string, IMonitoringQueryDefinition>
    chart?: unknown
    height?: number
}

export interface IMonitoringLayoutItem {
    widget: string
    width: number
    values?: Record<string, string>
}

export interface IMonitoringDashboardDefinition {
    id: string
    title: string
    requirements?: string[]
    order?: number
    selectors?: string[]
    theme?: string
    layout: IMonitoringLayoutItem[]
}

/** The dashboard selectors, in the order the header shows them. */
export enum EMonitoringScopeSelector {
    REGION = "region",
    POST = "post",
    COUNTRY = "country",
}

export const MONITORING_GRID_COLUMNS = 12

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Monitoring types, in one place so they can be swapped for the generated
// GraphQL types once the Hasura actions are in the schema. Field names follow
// the Harvest JSON (snake_case); configuration documents mirror
// `sequent_core::monitoring::config`.

/** How often the dashboard asks whether a new snapshot is live. */
export const MONITORING_DEFAULT_REFRESH_MS = 30_000

/** Frame height when the widget YAML sets none. */
export const DEFAULT_WIDGET_HEIGHT = 280

/** Chart widths are requested in steps of this many pixels, so renders cache. */
export const WIDTH_BUCKET_PX = 40

/** How long a resize settles before a new width is requested. */
export const WIDTH_DEBOUNCE_MS = 200

export const GRID_COLUMNS = 12

/** Result column the renderer orders by; the data table does not show it. */
export const POSITION_COLUMN = "position"

export enum EMonitoringMode {
    LEGACY = "LEGACY",
    CONFIGURED = "CONFIGURED",
}

/** Whether the dashboard is viewed or being edited; editing pauses polling. */
export enum EMonitoringViewMode {
    VIEW = "VIEW",
    EDIT = "EDIT",
}

export enum EMonitoringCapability {
    GRANTED = "GRANTED",
    DENIED = "DENIED",
}

export enum EScopeSelector {
    REGION = "region",
    POST = "post",
    COUNTRY = "country",
}

export const SCOPE_SELECTORS: EScopeSelector[] = [
    EScopeSelector.REGION,
    EScopeSelector.POST,
    EScopeSelector.COUNTRY,
]

export enum ESelectorControl {
    DROPDOWN = "dropdown",
    TOGGLE = "toggle",
}

export enum EDynamicOptions {
    EVENT_DAYS = "event_days",
}

export enum EDataSource {
    VOTER_TURNOUT = "voter_turnout",
    TEST_VOTING = "test_voting",
    ENROLLMENT_DECISIONS = "enrollment_decisions",
    VOTING_CREDENTIALS = "voting_credentials",
    POLL_STATUS = "poll_status",
    FINAL_TESTING_LOCKDOWN = "final_testing_lockdown",
    COUNTING_TRANSMISSION = "counting_transmission",
    VOTING_ENROLLMENT_ACTIVITY = "voting_enrollment_activity",
    ACCESS_SECURITY = "access_security",
    ATTACK_DETECTIONS = "attack_detections",
    HELPDESK = "helpdesk",
}

export enum EProducerState {
    CONNECTED = "CONNECTED",
    NOT_CONNECTED = "NOT_CONNECTED",
}

export enum EWidgetRenderState {
    RENDERED = "RENDERED",
    NOT_CONNECTED = "NOT_CONNECTED",
    NO_SNAPSHOT = "NO_SNAPSHOT",
    SCOPE_PENDING = "SCOPE_PENDING",
    RENDER_FAILED = "RENDER_FAILED",
    INVALID = "INVALID",
}

/** Why the portal itself could not show a widget, beside the server's states. */
export enum EWidgetFailure {
    REQUEST_FAILED = "REQUEST_FAILED",
}

export type MonitoringUnavailableState =
    | Exclude<EWidgetRenderState, EWidgetRenderState.RENDERED>
    | EWidgetFailure

export enum EColorScheme {
    LIGHT = "LIGHT",
    DARK = "DARK",
}

export enum EProblemSeverity {
    ERROR = "ERROR",
    WARNING = "WARNING",
}

export enum EMonitoringExportFormat {
    CSV = "CSV",
    SQL = "SQL",
}

export enum EColumnKind {
    TEXT = "text",
    INTEGER = "integer",
    NUMBER = "number",
}

export enum EConfigKind {
    DASHBOARD = "dashboard",
    WIDGET = "widget",
    THEME = "theme",
    SETTINGS = "settings",
}

export enum EConfigChange {
    UPSERT = "UPSERT",
    DELETE = "DELETE",
}

export enum EValidationResult {
    VALID = "VALID",
    INVALID = "INVALID",
}

// ---------------------------------------------------------------------
// Configuration documents (sequent_core::monitoring::config)
// ---------------------------------------------------------------------

/** `when: {selector: grain, in: [hour]}` */
export interface MonitoringCondition {
    selector: string
    in: string[]
}

export interface MonitoringSelector {
    label: string
    /** Value to label, in display order. */
    options?: Record<string, string>
    options_from?: EDynamicOptions
    default?: string
    control?: ESelectorControl
    when?: MonitoringCondition
    maps?: Record<string, unknown>
}

export interface MonitoringWidget {
    id: string
    title: string
    source: EDataSource | string
    requirements?: string[]
    follows?: EScopeSelector[]
    selectors?: Record<string, MonitoringSelector>
    query?: Record<string, unknown>
    queries?: Record<string, Record<string, unknown>>
    chart: unknown
    height?: number
}

export interface MonitoringLayoutItem {
    widget: string
    width: number
    values?: Record<string, string>
}

export interface MonitoringDashboard {
    id: string
    title: string
    requirements?: string[]
    order?: number
    selectors?: EScopeSelector[]
    theme?: string
    layout: MonitoringLayoutItem[]
}

/** `{label: Region, all: All regions}` */
export interface MonitoringSelectorWords {
    label: string
    all: string
}

// ---------------------------------------------------------------------
// Harvest responses
// ---------------------------------------------------------------------

export interface MonitoringSnapshot {
    revision: number
    as_of: string
    checked_at?: string | null
}

export interface MonitoringDashboardSummary {
    id: string
    title: string
    requirements: string[]
    widget_count: number
}

export interface MonitoringListDashboardsResponse {
    mode: EMonitoringMode
    dashboards: MonitoringDashboardSummary[]
    snapshot?: MonitoringSnapshot | null
}

export interface MonitoringScopeOption {
    key: string
    label: string
}

export interface MonitoringPostOption extends MonitoringScopeOption {
    region?: string | null
}

export interface MonitoringScopeOptions {
    regions: MonitoringScopeOption[]
    posts: MonitoringPostOption[]
    countries: MonitoringScopeOption[]
}

export interface MonitoringSettingsView {
    time_zone: string
    unknown_label?: string | null
    selectors?: Partial<Record<EScopeSelector, MonitoringSelectorWords>>
}

export interface MonitoringSourceInfo {
    counting_unit: string
    measures: string[]
    templates: string[]
    dimensions: string[]
    producer: EProducerState
    reason?: string | null
}

/** A widget definition as the server sends it; not yet checked. */
export interface MonitoringWidgetEntry {
    definition: unknown
    revision: number
}

export interface MonitoringGetDashboardResponse {
    dashboard: unknown
    dashboard_revision: number
    widgets: Record<string, MonitoringWidgetEntry>
    theme?: {id: string; revision: number} | null
    settings: MonitoringSettingsView
    settings_revision: number
    scope_options: MonitoringScopeOptions
    restricted: boolean
    pinned_post?: string | null
    sources: Record<string, MonitoringSourceInfo>
    snapshot?: MonitoringSnapshot | null
}

export interface MonitoringProblem {
    severity: EProblemSeverity
    code: string
    path: string
    message: string
    /** The dbt Charts code (`ERR-…`, `WARN-…`) behind a chart problem. */
    engine_code?: string | null
}

export interface MonitoringTableColumn {
    name: string
    kind: EColumnKind | string
}

export interface MonitoringTable {
    columns: MonitoringTableColumn[]
    rows: unknown[][]
}

export interface MonitoringScope {
    region?: string
    post?: string
    country?: string
}

export interface MonitoringRenderWidgetRequest {
    election_event_id: string
    election_id?: string
    dashboard_id: string
    widget_id: string
    scope: MonitoringScope
    selector_values: Record<string, string>
    snapshot_revision?: number
    width: number
    color_scheme: EColorScheme
    locale: string
    draft?: {widget_yaml?: string; theme_yaml?: string}
}

export interface MonitoringRenderWidgetResponse {
    state: EWidgetRenderState
    reason?: string | null
    svg?: string | null
    table?: MonitoringTable | null
    notices: string[]
    diagnostics: MonitoringProblem[]
    ignored_selectors: string[]
    render_ms?: number | null
    snapshot_revision?: number | null
    as_of?: string | null
}

export interface MonitoringTaskExecution {
    id: string
    name?: string
    execution_status?: string
}

export interface MonitoringExportRequest {
    election_event_id: string
    election_id?: string
    dashboard_id: string
    widget_id?: string
    scope: MonitoringScope
    selector_values: Record<string, string>
    snapshot_revision: number
    format: EMonitoringExportFormat
    from?: string
    to?: string
}

export interface MonitoringExportResponse {
    document_id: string
    task_execution?: MonitoringTaskExecution | null
}

export interface MonitoringValidateConfigResponse {
    result: EValidationResult
    problems: MonitoringProblem[]
    preview?: MonitoringRenderWidgetResponse | null
}

export interface MonitoringSaveConfigResponse {
    revision: number
    generation: number
    /** Problems that do not stop the save, such as a chart warning. */
    warnings: MonitoringProblem[]
}

export interface MonitoringConfigAuthor {
    id: string
    name?: string | null
}

export interface MonitoringConfigDocument {
    kind: EConfigKind
    key: string
    revision: number
    origin: string
    author?: MonitoringConfigAuthor | null
    created_at: string
    sha256?: string | null
}

export interface MonitoringConfigHistoryEntry {
    revision: number
    change: string
    origin: string
    author?: MonitoringConfigAuthor | null
    created_at: string
    sha256?: string | null
    generation: number
}

export interface MonitoringGetConfigResponse {
    kind: EConfigKind
    key: string
    /** `null` when the revision read is a removal. */
    yaml: string | null
    revision: number
    change: string
    origin: string
    author?: MonitoringConfigAuthor | null
    created_at: string
    history: MonitoringConfigHistoryEntry[]
}

export interface MonitoringPreset {
    id: string
    version: number
    title: string
    description?: string | null
}

// ---------------------------------------------------------------------
// Portal state
// ---------------------------------------------------------------------

export type MonitoringScopeValues = MonitoringScope

export interface MonitoringState {
    dashboardId: string | null
    dashboardValues: MonitoringScopeValues
    /** Viewer's picks per widget id, then selector name. */
    widgetValues: Record<string, Record<string, string>>
    mode: EMonitoringViewMode
}

// ---------------------------------------------------------------------
// GraphQL operation results (Hasura actions over the Harvest routes)
// ---------------------------------------------------------------------

export interface MonitoringListDashboardsQuery {
    monitoringListDashboards: MonitoringListDashboardsResponse
}

export interface MonitoringListDashboardsVariables {
    electionEventId: string
    electionId?: string | null
}

export interface MonitoringGetDashboardQuery {
    monitoringGetDashboard: MonitoringGetDashboardResponse
}

export interface MonitoringGetDashboardVariables extends MonitoringListDashboardsVariables {
    dashboardId: string
}

export interface MonitoringRenderWidgetQuery {
    monitoringRenderWidget: MonitoringRenderWidgetResponse
}

export interface MonitoringRenderWidgetVariables extends MonitoringGetDashboardVariables {
    widgetId: string
    scope: MonitoringScope
    selectorValues: Record<string, string>
    snapshotRevision?: number | null
    width: number
    colorScheme: EColorScheme
    locale: string
    draft?: {widget_yaml?: string; theme_yaml?: string} | null
}

export interface MonitoringExportMutation {
    monitoringExport: MonitoringExportResponse
}

export interface MonitoringExportVariables extends MonitoringGetDashboardVariables {
    widgetId?: string | null
    scope: MonitoringScope
    selectorValues: Record<string, string>
    snapshotRevision: number
    format: EMonitoringExportFormat
    from?: string | null
    to?: string | null
}

export interface MonitoringValidateConfigQuery {
    monitoringValidateConfig: MonitoringValidateConfigResponse
}

export interface MonitoringSaveConfigMutation {
    monitoringSaveConfig: MonitoringSaveConfigResponse
}

export interface MonitoringResetToPresetMutation {
    monitoringResetToPreset: {generation: number; warnings?: MonitoringProblem[] | null}
}

export interface MonitoringListPresetsQuery {
    monitoringListPresets: {presets: MonitoringPreset[]}
}

export interface MonitoringSetModeMutation {
    monitoringSetMode: {mode: EMonitoringMode; generation: number}
}

export interface MonitoringListConfigQuery {
    monitoringListConfig: {
        mode: EMonitoringMode
        generation: number
        preset?: {id: string; version: number} | null
        documents: MonitoringConfigDocument[]
    }
}

export interface MonitoringGetConfigQuery {
    monitoringGetConfig: MonitoringGetConfigResponse
}

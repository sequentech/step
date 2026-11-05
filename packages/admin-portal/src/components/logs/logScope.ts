// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** Which log a list, an entry popup or an export is about. */
export enum ELogScope {
    /** An election event's log. */
    EVENT = "event",
    /** The tenant log (Settings > Logs). */
    TENANT = "tenant",
}

/** One log: an election event's, by its id, or the tenant's. */
export type ILogTarget = {kind: ELogScope.EVENT; electionEventId: string} | {kind: ELogScope.TENANT}

/** The react-admin `meta` key that carries the scope to the data provider. */
export const LOG_SCOPE_META = "logScope"

/** The statement kinds the tenant log holds. */
export const TENANT_LOG_STATEMENT_KINDS = [
    "KeycloakUserEvent",
    "AdminChange",
    "TenantAdminAction",
    "KeycloakAdminEvent",
    "LogsExported",
    "LogWriterStarted",
    "LogWriterStopped",
    "AdminPublicKey",
] as const

export const LOG_EVENT_TYPES = ["USER", "SYSTEM"] as const
export const LOG_TYPES = ["INFO", "ERROR"] as const

export const LOG_FILTER_EVENT_TYPE = "event_type"
export const LOG_FILTER_LOG_TYPE = "log_type"

/** Below this width of the list, Created starts hidden under Columns. */
export const LOG_LIST_WIDE = 1500
/** Below this width of the list, each entry is two lines. */
export const LOG_LIST_COMPACT = 1000

export enum ELogListLayout {
    WIDE = "wide",
    MEDIUM = "medium",
    COMPACT = "compact",
}

/** The layout of a list that is `width` pixels wide; wide until it is measured. */
export const logListLayout = (width: number | null): ELogListLayout => {
    if (width === null || width >= LOG_LIST_WIDE) return ELogListLayout.WIDE
    return width >= LOG_LIST_COMPACT ? ELogListLayout.MEDIUM : ELogListLayout.COMPACT
}

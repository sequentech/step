// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ELogScope} from "./logScope"

export enum EExportLogsFormat {
    CSV = "CSV",
    SQL = "SQL",
    // The PDF arrives zipped.
    PDF = "PDF",
}

/** What an export covers, as the export actions name it. */
export enum EExportLogsScope {
    /** The whole tenant log. */
    TENANT_LOG = "TENANT_LOG",
    /** One account's entries. */
    ACCOUNT = "ACCOUNT",
    /** One account's entries in the tenant log and in every election event's log. */
    ACCOUNT_WITH_EVENTS = "ACCOUNT_WITH_EVENTS",
    /** The whole election event. */
    ALL = "ALL",
    ELECTION = "ELECTION",
    AREA = "AREA",
}

export interface IExportLogsChoice {
    scope: EExportLogsScope
    username: string
    electionId: string | null
    areaId: string | null
}

export interface IExportLogsRange {
    createdFrom: string | null
    createdTo: string | null
    timeZone: string | null
}

const SCOPES: Record<ELogScope, EExportLogsScope[]> = {
    [ELogScope.TENANT]: [EExportLogsScope.TENANT_LOG, EExportLogsScope.ACCOUNT],
    [ELogScope.EVENT]: [
        EExportLogsScope.ALL,
        EExportLogsScope.ELECTION,
        EExportLogsScope.AREA,
        EExportLogsScope.ACCOUNT,
    ],
}

/** The scopes the export dialog offers for a log; the first is its default. */
export const exportLogsScopes = (log: ELogScope): EExportLogsScope[] => SCOPES[log]

const ACCOUNT_SCOPES = [EExportLogsScope.ACCOUNT, EExportLogsScope.ACCOUNT_WITH_EVENTS]

export const isAccountScope = (scope: EExportLogsScope): boolean => ACCOUNT_SCOPES.includes(scope)

/** One account of the tenant log, with or without its entries in every election event. */
export const tenantExportScope = (
    scope: EExportLogsScope,
    withElectionEvents: boolean
): EExportLogsScope =>
    isAccountScope(scope)
        ? withElectionEvents
            ? EExportLogsScope.ACCOUNT_WITH_EVENTS
            : EExportLogsScope.ACCOUNT
        : scope

/** Whether the scope has what it names: an account, an election or an area. */
export const exportScopeReady = ({
    scope,
    username,
    electionId,
    areaId,
}: IExportLogsChoice): boolean => {
    if (isAccountScope(scope)) return !!username.trim()
    if (scope === EExportLogsScope.ELECTION) return !!electionId
    if (scope === EExportLogsScope.AREA) return !!areaId
    return true
}

const usernameOf = ({scope, username}: IExportLogsChoice): string | null =>
    isAccountScope(scope) ? username.trim() : null

/** The variables of `export_tenant_logs`. */
export const tenantExportVariables = (
    choice: IExportLogsChoice,
    format: EExportLogsFormat,
    range: IExportLogsRange
) => ({
    format,
    scope: choice.scope,
    username: usernameOf(choice),
    ...range,
})

/** The variables of `export_election_event_logs`. */
export const eventExportVariables = (
    electionEventId: string,
    choice: IExportLogsChoice,
    format: EExportLogsFormat,
    range: IExportLogsRange
) => ({
    electionEventId,
    format,
    scope: choice.scope,
    electionId: choice.scope === EExportLogsScope.ELECTION ? choice.electionId : null,
    areaId: choice.scope === EExportLogsScope.AREA ? choice.areaId : null,
    username: usernameOf(choice),
    ...range,
})

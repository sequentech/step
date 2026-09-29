// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Harvest answers a refused monitoring request with `{message, extensions:
// {code, ...}}`, which Hasura passes on as the GraphQL error's extensions (or,
// in some setups, keeps in `extensions.internal.response.body`).

/** The codes the view tells the viewer about. */
export enum EMonitoringErrorCode {
    /** 503 with Retry-After: the event is busy; asked again after a moment. */
    BUSY = "MONITORING_BUSY",
    /** 403: a region, Post or country outside what the viewer may see. */
    FORBIDDEN_SCOPE = "MONITORING_FORBIDDEN_SCOPE",
    /** 410: the update being exported is no longer kept. */
    SNAPSHOT_PRUNED = "MONITORING_SNAPSHOT_PRUNED",
    /** 503: the chart engine is not answering. */
    CHECKS_UNAVAILABLE = "MONITORING_CHECKS_UNAVAILABLE",
    /** 403: the event is locked down. */
    LOCKED_DOWN = "MONITORING_LOCKED_DOWN",
    /** 404: the dashboard or widget is no longer configured. */
    NOT_FOUND = "MONITORING_NOT_FOUND",
    /** 400 or 422: the request is not one Harvest accepts, such as a bound without an offset. */
    BAD_REQUEST = "MONITORING_BAD_REQUEST",
    /** 422 with `problems`: values Harvest checked and refused, such as a widget's pick. */
    INVALID = "MONITORING_INVALID",
}

/** How long a busy answer waits before it is asked again. */
export const BUSY_RETRY_MS = 3_000

const CODES = new Set<string>(Object.values(EMonitoringErrorCode))

const MESSAGES: Record<EMonitoringErrorCode, string> = {
    [EMonitoringErrorCode.BUSY]: "monitoring.errors.busy",
    [EMonitoringErrorCode.FORBIDDEN_SCOPE]: "monitoring.errors.forbiddenScope",
    [EMonitoringErrorCode.SNAPSHOT_PRUNED]: "monitoring.errors.snapshotPruned",
    [EMonitoringErrorCode.CHECKS_UNAVAILABLE]: "monitoring.errors.checksUnavailable",
    [EMonitoringErrorCode.LOCKED_DOWN]: "monitoring.errors.lockedDown",
    [EMonitoringErrorCode.NOT_FOUND]: "monitoring.errors.notFound",
    [EMonitoringErrorCode.BAD_REQUEST]: "monitoring.errors.badRequest",
    [EMonitoringErrorCode.INVALID]: "monitoring.errors.invalid",
}

const record = (value: unknown): Record<string, unknown> =>
    value && typeof value === "object" ? (value as Record<string, unknown>) : {}

function parsed(body: unknown): Record<string, unknown> {
    if (typeof body !== "string") return record(body)
    try {
        return record(JSON.parse(body))
    } catch {
        return {}
    }
}

/** Each error's extensions, and those of the handler's body when Hasura wraps it. */
function extensionsOf(error: unknown): Record<string, unknown>[] {
    const graphQLErrors = record(error).graphQLErrors
    if (!Array.isArray(graphQLErrors)) return []
    return graphQLErrors.flatMap((graphQLError) => {
        const extensions = record(record(graphQLError).extensions)
        const body = parsed(record(record(extensions.internal).response).body)
        return [extensions, record(body.extensions)]
    })
}

/** The codes in an error, wherever Hasura put them. */
function codesOf(error: unknown): unknown[] {
    return extensionsOf(error).map((extensions) => extensions.code)
}

/** A problem Harvest found in a request: a code, where, and its words for it. */
export interface MonitoringRequestProblem {
    code: string
    path: string
    message: string
}

/** The `problems` of a MONITORING_INVALID refusal. */
export function monitoringProblems(error: unknown): MonitoringRequestProblem[] {
    return extensionsOf(error).flatMap((extensions) =>
        Array.isArray(extensions.problems)
            ? extensions.problems.flatMap((problem): MonitoringRequestProblem[] => {
                  const {code, path, message} = record(problem)
                  return typeof code === "string"
                      ? [
                            {
                                code,
                                path: typeof path === "string" ? path : "",
                                message: typeof message === "string" ? message : "",
                            },
                        ]
                      : []
              })
            : []
    )
}

/** Harvest's code for a failed monitoring request, when it is one the view knows. */
export function monitoringErrorCode(error: unknown): EMonitoringErrorCode | undefined {
    return codesOf(error).find(
        (code): code is EMonitoringErrorCode => typeof code === "string" && CODES.has(code)
    )
}

/** The translation key of what to tell the viewer about a failed request. */
export function monitoringErrorMessage(error: unknown): string {
    const code = monitoringErrorCode(error)
    return code ? MESSAGES[code] : "monitoring.errors.unknown"
}

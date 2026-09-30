// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What the editor asks of Harvest. Components take this interface rather
 * than calling Apollo themselves, so stories and tests hand them a fake and
 * the production hook ({@link useMonitoringEditorApi}) is the only place that
 * knows about GraphQL.
 */

import {parseActionResponseBody} from "@/services/graphqlActionError"
import type {IGraphQLActionError} from "@sequentech/ui-core"
import {normalizeProblems} from "@/components/monitoring/lib/problems"
import {EMonitoringErrorCode} from "@/components/monitoring/lib/errors"
import {
    EMonitoringConfigKind,
    EMonitoringSaveStatus,
    type IMonitoringAuthor,
    type IMonitoringConfigDocument,
    type IMonitoringConfigListEntry,
    type IMonitoringPreset,
    type IMonitoringRenderRequest,
    type IMonitoringRenderResponse,
    type IMonitoringSaveRequest,
    type IMonitoringValidateResponse,
    type TMonitoringSaveOutcome,
} from "./types"

export interface IMonitoringEditorApi {
    validateConfig(request: {
        kind: EMonitoringConfigKind
        key: string
        yaml: string
    }): Promise<IMonitoringValidateResponse>
    /** Resolves with the outcome for a stored, conflicting or invalid save; rejects on anything else. */
    saveConfig(request: IMonitoringSaveRequest): Promise<TMonitoringSaveOutcome>
    renderWidget(request: IMonitoringRenderRequest): Promise<IMonitoringRenderResponse>
    getConfig(request: {
        kind: EMonitoringConfigKind
        key: string
        revision?: number
    }): Promise<IMonitoringConfigDocument>
    listConfig(): Promise<IMonitoringConfigListEntry[]>
    listPresets(): Promise<IMonitoringPreset[]>
    resetToPreset(presetId: string): Promise<{generation: number}>
}

/** Earlier spellings, still read so an older Harvest keeps working. */
const CONFLICT_CODES = new Set<string>([EMonitoringErrorCode.CONFLICT, "conflict", "CONFLICT"])
const INVALID_CODES = new Set<string>([
    EMonitoringErrorCode.INVALID,
    "invalid",
    "INVALID",
    "unprocessable",
    "validation-failed",
])
const HARVEST_CODES = new Set<string>(Object.values(EMonitoringErrorCode))
const CONFLICT_STATUS = 409
const INVALID_STATUS = 422

/** What the editor says for a refusal that needs no more than a message. */
const ERROR_MESSAGES: Partial<Record<string, string>> = {
    [EMonitoringErrorCode.CHECKS_UNAVAILABLE]: "monitoring.editor.errors.checksUnavailable",
    [EMonitoringErrorCode.BUSY]: "monitoring.editor.errors.busy",
    [EMonitoringErrorCode.LOCKED_DOWN]: "monitoring.editor.errors.lockedDown",
    [EMonitoringErrorCode.FORBIDDEN_SCOPE]: "monitoring.editor.errors.forbiddenScope",
    [EMonitoringErrorCode.BAD_REQUEST]: "monitoring.editor.errors.badRequest",
}

const record = (value: unknown): Record<string, unknown> =>
    value && typeof value === "object" ? (value as Record<string, unknown>) : {}

const authorOf = (value: unknown): IMonitoringAuthor | null => {
    if (typeof value === "string") return {id: value}
    const author = record(value)
    return typeof author.id === "string"
        ? {id: author.id, name: typeof author.name === "string" ? author.name : null}
        : null
}

interface IRefusal {
    code: string
    status?: number
    details: Record<string, unknown>
}

/**
 * Each refusal in a failed action, wherever Hasura put Harvest's answer
 * (`{message, extensions: {code, ...}}`): promoted into the GraphQL error's
 * extensions, or left in the original response body.
 */
const refusals = (error: unknown): IRefusal[] => {
    const actionError = error as IGraphQLActionError | undefined
    return (actionError?.graphQLErrors ?? []).map((graphQLError) => {
        const extensions = record(graphQLError.extensions)
        const internal = record(extensions.internal)
        const response = record(internal.response)
        const body = record(parseActionResponseBody(response.body as string | undefined))
        const bodyExtensions = record(body.extensions)
        const promoted = typeof extensions.code === "string" && extensions.code !== "unexpected"
        return {
            code: String(
                (promoted ? extensions.code : bodyExtensions.code) ?? extensions.code ?? ""
            ),
            status: typeof response.status === "number" ? response.status : undefined,
            details: {...body, ...bodyExtensions, ...extensions},
        }
    })
}

/** Harvest's code for a failed action, when it gave one. */
export const monitoringErrorCode = (error: unknown): string | undefined =>
    refusals(error).find((refusal) => refusal.code)?.code

/**
 * The translation key of what to tell the author about a failure Harvest
 * explained (busy, locked down, …); `undefined` when the reason is to be shown as is.
 */
export const monitoringErrorMessage = (error: unknown): string | undefined => {
    const code = monitoringErrorCode(error)
    return code ? ERROR_MESSAGES[code] : undefined
}

const revisionOf = (value: unknown): number | null => {
    if (value === null || value === undefined || value === "") return null
    const revision = Number(value)
    return Number.isFinite(revision) ? revision : null
}

/**
 * A save Harvest refused: 409 carries `{current_revision, author, time}`
 * and 422 `{problems}`. `undefined` for any other failure.
 */
export const interpretSaveError = (error: unknown): TMonitoringSaveOutcome | undefined => {
    for (const {code, status, details} of refusals(error)) {
        if (CONFLICT_CODES.has(code) || status === CONFLICT_STATUS) {
            return {
                status: EMonitoringSaveStatus.CONFLICT,
                current_revision: revisionOf(details.current_revision),
                author: authorOf(details.author),
                time: typeof details.time === "string" ? details.time : null,
            }
        }
        // A 422 is read as problems only when Harvest named no code of its
        // own: a MONITORING_BAD_REQUEST is a request it could not read, not
        // a document it found problems in.
        if (INVALID_CODES.has(code) || (status === INVALID_STATUS && !HARVEST_CODES.has(code))) {
            return {
                status: EMonitoringSaveStatus.INVALID,
                problems: normalizeProblems(details.problems),
            }
        }
    }
    return undefined
}

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
import {normalizeProblems} from "@/components/monitoring/lib/diagnostics"
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

const CONFLICT_CODES = new Set(["conflict", "CONFLICT", "409"])
const INVALID_CODES = new Set(["invalid", "INVALID", "422", "unprocessable", "validation-failed"])
const CONFLICT_STATUS = 409
const INVALID_STATUS = 422

const record = (value: unknown): Record<string, unknown> =>
    value && typeof value === "object" ? (value as Record<string, unknown>) : {}

const authorOf = (value: unknown): IMonitoringAuthor | null => {
    if (typeof value === "string") return {id: value}
    const author = record(value)
    return typeof author.id === "string"
        ? {id: author.id, name: typeof author.name === "string" ? author.name : null}
        : null
}

/**
 * A save Harvest refused, wherever Hasura put the reason: 409 carries
 * `{current_revision, author, time}` and 422 `{problems}`, either promoted
 * into the error's extensions or left in the original response body.
 * `undefined` for any other failure.
 */
export const interpretSaveError = (error: unknown): TMonitoringSaveOutcome | undefined => {
    const actionError = error as IGraphQLActionError | undefined
    for (const graphQLError of actionError?.graphQLErrors ?? []) {
        const extensions = record(graphQLError.extensions)
        const internal = record(extensions.internal)
        const response = record(internal.response)
        const body = record(parseActionResponseBody(response.body as string | undefined))
        const bodyExtensions = record(body.extensions)
        const code = String(extensions.code ?? bodyExtensions.code ?? "")
        const status = typeof response.status === "number" ? response.status : undefined
        const details = {...body, ...bodyExtensions, ...extensions}
        if (CONFLICT_CODES.has(code) || status === CONFLICT_STATUS) {
            return {
                status: EMonitoringSaveStatus.CONFLICT,
                current_revision: Number(details.current_revision ?? 0),
                author: authorOf(details.author),
                time: typeof details.time === "string" ? details.time : null,
            }
        }
        if (INVALID_CODES.has(code) || status === INVALID_STATUS) {
            return {
                status: EMonitoringSaveStatus.INVALID,
                problems: normalizeProblems(details.problems),
            }
        }
    }
    return undefined
}

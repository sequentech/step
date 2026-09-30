// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useMemo} from "react"
import {useApolloClient, type DocumentNode} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {normalizeProblems} from "@/components/monitoring/lib/problems"
import {interpretSaveError, type IMonitoringEditorApi} from "./api"
import {
    MONITORING_EDITOR_GET_CONFIG,
    MONITORING_EDITOR_LIST_CONFIG,
    MONITORING_EDITOR_LIST_PRESETS,
    MONITORING_EDITOR_RENDER_WIDGET,
    MONITORING_EDITOR_RESET_TO_PRESET,
    MONITORING_EDITOR_SAVE_CONFIG,
    MONITORING_EDITOR_VALIDATE_CONFIG,
} from "./queries"
import {
    EMonitoringSaveStatus,
    EMonitoringValidationResult,
    type IMonitoringAuthor,
    type IMonitoringConfigListEntry,
    type IMonitoringPreset,
    type IMonitoringRenderResponse,
} from "./types"

const asRender = (value: unknown): IMonitoringRenderResponse => {
    const response = (value ?? {}) as IMonitoringRenderResponse
    return {...response, diagnostics: normalizeProblems(response.diagnostics)}
}

interface IConfigReply {
    yaml?: string | null
    revision: number
    origin?: string | null
    author?: IMonitoringAuthor | null
    created_at?: string | null
}

/** Harvest's monitoring actions, as the editor's components need them; all run as `monitoring-configure`. */
export const useMonitoringEditorApi = (electionEventId: string): IMonitoringEditorApi => {
    const client = useApolloClient()
    return useMemo(() => {
        const context = {headers: {"x-hasura-role": IPermissions.MONITORING_CONFIGURE}}
        const first = <T>(data: Record<string, T> | null | undefined): T => {
            const [value] = Object.values(data ?? {})
            if (value === undefined || value === null) {
                throw new Error("The action returned no data")
            }
            return value
        }
        const mutate = async <T>(mutation: DocumentNode, variables: Record<string, unknown>) => {
            const {data} = await client.mutate<Record<string, T>>({
                mutation,
                variables: {election_event_id: electionEventId, ...variables},
                context,
            })
            return first(data)
        }
        const query = async <T>(document: DocumentNode, variables: Record<string, unknown>) => {
            const {data} = await client.query<Record<string, T>>({
                query: document,
                variables: {election_event_id: electionEventId, ...variables},
                fetchPolicy: "network-only",
                context,
            })
            return first(data)
        }

        const api: IMonitoringEditorApi = {
            async validateConfig({kind, key, yaml}) {
                const reply = await query<{result: string; problems: unknown; preview: unknown}>(
                    MONITORING_EDITOR_VALIDATE_CONFIG,
                    {kind, key, yaml}
                )
                return {
                    result:
                        reply.result === EMonitoringValidationResult.VALID
                            ? EMonitoringValidationResult.VALID
                            : EMonitoringValidationResult.INVALID,
                    problems: normalizeProblems(reply.problems),
                    preview: reply.preview ? asRender(reply.preview) : null,
                }
            },
            async saveConfig({kind, key, yaml, expected_revision, change}) {
                try {
                    const reply = await mutate<{
                        revision: number
                        generation: number
                        warnings?: unknown
                        author?: IMonitoringAuthor | null
                        created_at?: string | null
                    }>(MONITORING_EDITOR_SAVE_CONFIG, {
                        kind,
                        key,
                        yaml: yaml ?? null,
                        expected_revision: expected_revision ?? null,
                        change,
                    })
                    return {
                        status: EMonitoringSaveStatus.SAVED,
                        revision: reply.revision,
                        generation: reply.generation,
                        warnings: normalizeProblems(reply.warnings),
                        author: reply.author,
                        created_at: reply.created_at,
                    }
                } catch (error) {
                    const outcome = interpretSaveError(error)
                    if (outcome) return outcome
                    throw error
                }
            },
            async renderWidget(request) {
                return asRender(
                    await query(MONITORING_EDITOR_RENDER_WIDGET, {
                        ...request,
                        election_id: request.election_id ?? null,
                        draft: request.draft ?? null,
                    })
                )
            },
            async getConfig({kind, key, revision}) {
                const reply = await query<IConfigReply>(MONITORING_EDITOR_GET_CONFIG, {
                    kind,
                    key,
                    revision: revision ?? null,
                })
                return {
                    kind,
                    key,
                    yaml: reply.yaml ?? null,
                    revision: reply.revision,
                    origin: reply.origin,
                    author: reply.author,
                    created_at: reply.created_at,
                }
            },
            async listConfig() {
                const reply = await query<{documents: IMonitoringConfigListEntry[] | null}>(
                    MONITORING_EDITOR_LIST_CONFIG,
                    {}
                )
                return reply.documents ?? []
            },
            async listPresets() {
                const reply = await query<{presets: IMonitoringPreset[] | null}>(
                    MONITORING_EDITOR_LIST_PRESETS,
                    {}
                )
                return reply.presets ?? []
            },
            async resetToPreset(presetId) {
                const reply = await mutate<{generation: number}>(
                    MONITORING_EDITOR_RESET_TO_PRESET,
                    {preset_id: presetId}
                )
                return {generation: reply.generation}
            },
        }
        return api
    }, [client, electionEventId])
}

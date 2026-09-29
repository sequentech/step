// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useMemo} from "react"
import {useApolloClient, type DocumentNode} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {normalizeProblems} from "@/components/monitoring/lib/diagnostics"
import {interpretSaveError, type IMonitoringEditorApi} from "./api"
import {
    MONITORING_GET_CONFIG,
    MONITORING_LIST_CONFIG,
    MONITORING_LIST_PRESETS,
    MONITORING_RENDER_WIDGET_DRAFT,
    MONITORING_RESET_TO_PRESET,
    MONITORING_SAVE_CONFIG,
    MONITORING_VALIDATE_CONFIG,
} from "./queries"
import {
    EMonitoringSaveStatus,
    EMonitoringValidationResult,
    type IMonitoringConfigDocument,
    type IMonitoringConfigListEntry,
    type IMonitoringPreset,
    type IMonitoringRenderResponse,
} from "./types"

const asRender = (value: unknown): IMonitoringRenderResponse => {
    const response = (value ?? {}) as IMonitoringRenderResponse
    return {...response, diagnostics: normalizeProblems(response.diagnostics)}
}

/** Harvest's monitoring actions, as the editor's components need them. */
export const useMonitoringEditorApi = (electionEventId: string): IMonitoringEditorApi => {
    const client = useApolloClient()
    return useMemo(() => {
        const context = (role: IPermissions) => ({headers: {"x-hasura-role": role}})
        const mutate = async <T>(
            mutation: DocumentNode,
            variables: Record<string, unknown>,
            role = IPermissions.MONITORING_CONFIGURE
        ) => {
            const {data} = await client.mutate<Record<string, T>>({
                mutation,
                variables: {election_event_id: electionEventId, ...variables},
                context: context(role),
            })
            const [value] = Object.values(data ?? {})
            if (value === undefined) throw new Error("The action returned no data")
            return value
        }
        const query = async <T>(document: DocumentNode, variables: Record<string, unknown>) => {
            const {data} = await client.query<Record<string, T>>({
                query: document,
                variables: {election_event_id: electionEventId, ...variables},
                fetchPolicy: "network-only",
                context: context(IPermissions.MONITORING_CONFIGURE),
            })
            const [value] = Object.values(data ?? {})
            if (value === undefined) throw new Error("The action returned no data")
            return value
        }

        const api: IMonitoringEditorApi = {
            async validateConfig({kind, key, yaml}) {
                const reply = await mutate<{result: string; problems: unknown; preview: unknown}>(
                    MONITORING_VALIDATE_CONFIG,
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
            async saveConfig(request) {
                try {
                    const reply = await mutate<{revision: number; generation: number}>(
                        MONITORING_SAVE_CONFIG,
                        {...request}
                    )
                    return {
                        status: EMonitoringSaveStatus.SAVED,
                        revision: reply.revision,
                        generation: reply.generation,
                    }
                } catch (error) {
                    const outcome = interpretSaveError(error)
                    if (outcome) return outcome
                    throw error
                }
            },
            async renderWidget(request) {
                return asRender(
                    await mutate(
                        MONITORING_RENDER_WIDGET_DRAFT,
                        {...request},
                        IPermissions.MONITORING_CONFIGURE
                    )
                )
            },
            async getConfig({kind, key, revision}) {
                const reply = await query<Omit<IMonitoringConfigDocument, "kind" | "key">>(
                    MONITORING_GET_CONFIG,
                    {kind, key, revision}
                )
                return {...reply, kind, key}
            },
            async listConfig() {
                const reply = await query<{documents: IMonitoringConfigListEntry[] | null}>(
                    MONITORING_LIST_CONFIG,
                    {}
                )
                return reply.documents ?? []
            },
            async listPresets() {
                const reply = await query<{presets: IMonitoringPreset[] | null}>(
                    MONITORING_LIST_PRESETS,
                    {}
                )
                return reply.presets ?? []
            },
            async resetToPreset(presetId) {
                return mutate<{generation: number}>(MONITORING_RESET_TO_PRESET, {
                    preset_id: presetId,
                })
            },
        }
        return api
    }, [client, electionEventId])
}

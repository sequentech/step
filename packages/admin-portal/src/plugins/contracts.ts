// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {ComponentType, LazyExoticComponent} from "react"

export enum PluginState {
    Enabled = "enabled",
    Disabled = "disabled",
}
export enum Compatibility {
    Compatible = "compatible",
    Incompatible = "incompatible",
}
/** Stable gateway error codes, independent of the message language. */
export enum PluginErrorCode {
    Forbidden = "plugin-forbidden",
    Disabled = "plugin-disabled",
    Incompatible = "plugin-incompatible",
    NotFound = "plugin-not-found",
    Conflict = "plugin-conflict",
    StateConflict = "plugin-state-conflict",
    InvalidInput = "plugin-invalid-input",
    Unavailable = "plugin-unavailable",
}

export interface PluginManifest {
    id: string
    version: string
    api_major: number
    label_key: string
    route: string
    read_permission: string
    permissions: string[]
}
export interface TenantPlugin {
    manifest: PluginManifest
    state: PluginState
    version: number
    compatibility: Compatibility
}
/** What the host gives a plugin's screen: its own operations and the user's permissions. */
export interface PluginHost {
    execute: <T>(operation: string, data: Record<string, unknown>) => Promise<T>
    can: (permission: string) => boolean
}
export interface PluginContribution {
    id: string
    apiMajor: number
    labelKey: string
    route: string
    component: LazyExoticComponent<ComponentType<{host: PluginHost}>>
}
export class PluginRequestError extends Error {
    constructor(
        message: string,
        public code: PluginErrorCode,
        public current?: unknown
    ) {
        super(message)
    }
}
export const errorMessage = (error: unknown): string =>
    error instanceof Error ? error.message : String(error)

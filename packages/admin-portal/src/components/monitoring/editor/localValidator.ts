// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * sequent-core's monitoring policy, run in the browser on every keystroke.
 *
 * Feature-detected: a sequent-core build without the monitoring export (or
 * whose WebAssembly is not loaded yet) yields no local checks, and the editor
 * relies on the server's, which run the same policy.
 */

import {normalizeProblems} from "@/components/monitoring/lib/diagnostics"
import type {TLocalValidate} from "./yamlDraft"
import type {EMonitoringConfigKind} from "./types"

export const WASM_VALIDATE_EXPORT = "validateMonitoringConfig"

type TWasmValidate = (kind: string, key: string, yaml: string, configSetJson: string) => unknown

const findExport = (module: object): TWasmValidate | undefined => {
    const candidate: unknown = Reflect.get(module, WASM_VALIDATE_EXPORT)
    return typeof candidate === "function" ? (candidate as TWasmValidate) : undefined
}

export interface ILocalValidatorOptions {
    /** The key the document is stored under; empty for a new document. */
    key?: string
    /** The event's documents; the draft takes the place of the one at `key`. */
    configSet?: unknown
}

/**
 * A validator for documents of `kind`, checked alone and, when `configSet`
 * is given, against the event's other documents, the draft replacing the
 * document stored under `key` (so a changed id is reported, not duplicated).
 * `null` from the validator means the module has no such export.
 */
export const createLocalValidator = (
    module: object,
    kind: EMonitoringConfigKind,
    {key = "", configSet}: ILocalValidatorOptions = {}
): TLocalValidate => {
    const configSetJson = configSet === undefined ? "" : JSON.stringify(configSet)
    return (text) => {
        const validate = findExport(module)
        if (!validate) return null
        const report = validate(kind, key, text, configSetJson) as {problems?: unknown} | unknown[]
        return normalizeProblems(Array.isArray(report) ? report : report?.problems)
    }
}

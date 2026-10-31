// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {validateSnapshot, type ScenarioSnapshot} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {parsePolicyOverrides, type PolicyOverrides} from "./policies"

/** Every key the workbench writes starts with this prefix, so a reset can find them all. */
export const STORAGE_PREFIX = "sequent.workbench.v1."
export const STATE_KEY = `${STORAGE_PREFIX}state`

export type StorageArea = Pick<
    Storage,
    "getItem" | "setItem" | "removeItem" | "key" | "length" | "clear"
>

/** What survives a reload: the loaded snapshot and the local policy overrides. */
export interface PersistedState {
    base: ScenarioSnapshot
    overrides: PolicyOverrides
}

/** The stored state, if any; a stored value that no longer validates throws. */
export function readPersistedState(storage: StorageArea): PersistedState | undefined {
    const stored = storage.getItem(STATE_KEY)
    if (stored === null) return undefined
    const value = JSON.parse(stored) as {base?: unknown; overrides?: unknown}
    return {base: validateSnapshot(value.base), overrides: parsePolicyOverrides(value.overrides)}
}

export function writePersistedState(storage: StorageArea, state: PersistedState) {
    storage.setItem(STATE_KEY, JSON.stringify(state))
}

/** Removes the workbench keys and the portal's session storage; both belong to this origin. */
export function clearWorkbenchStorage(local: StorageArea, session: StorageArea) {
    const keys = Array.from({length: local.length}, (_, index) => local.key(index))
    for (const key of keys) if (key?.startsWith(STORAGE_PREFIX)) local.removeItem(key)
    session.clear()
}

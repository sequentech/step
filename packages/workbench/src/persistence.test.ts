// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, test} from "vitest"
import {EUnderVotePolicy} from "@sequentech/ui-core"
import {ScenarioId, scenarioSnapshot} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {
    clearWorkbenchStorage,
    readPersistedState,
    STATE_KEY,
    writePersistedState,
    type StorageArea,
} from "./persistence"

/** An in-memory Storage with the Web Storage key order. */
function memoryStorage(entries: Record<string, string> = {}): StorageArea {
    const items = new Map(Object.entries(entries))
    return {
        get length() {
            return items.size
        },
        key: (index) => [...items.keys()][index] ?? null,
        getItem: (key) => items.get(key) ?? null,
        setItem: (key, value) => void items.set(key, value),
        removeItem: (key) => void items.delete(key),
        clear: () => items.clear(),
    }
}

test("the stored state is namespaced and round-trips", () => {
    const storage = memoryStorage()
    const state = {
        base: scenarioSnapshot(ScenarioId.KIOSK_VOTER),
        overrides: {contest: {under_vote_policy: EUnderVotePolicy.WARN}},
    }
    expect(readPersistedState(storage)).toBeUndefined()
    writePersistedState(storage, state)
    expect(STATE_KEY).toBe("sequent.workbench.v1.state")
    expect(storage.length).toBe(1)
    expect(storage.key(0)).toBe(STATE_KEY)
    expect(readPersistedState(storage)).toEqual(state)
})

test("a stored state that no longer validates is rejected", () => {
    const valid = JSON.stringify({
        base: scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY),
        overrides: {},
    })
    expect(readPersistedState(memoryStorage({[STATE_KEY]: valid}))).toBeDefined()
    expect(() => readPersistedState(memoryStorage({[STATE_KEY]: "{"}))).toThrow(SyntaxError)
    expect(() =>
        readPersistedState(
            memoryStorage({[STATE_KEY]: valid.replace('"version":1', '"version":2')})
        )
    ).toThrow("version: expected 1, found 2")
    expect(() =>
        readPersistedState(
            memoryStorage({
                [STATE_KEY]: JSON.stringify({...JSON.parse(valid), overrides: {c: {x: 1}}}),
            })
        )
    ).toThrow("Unsupported override x = 1")
})

test("clearing removes the workbench keys and the portal session only", () => {
    const local = memoryStorage({
        "sequent.workbench.v1.state": "{}",
        "sequent.workbench.v1.other": "x",
        "unrelated": "kept",
    })
    const session = memoryStorage({isDemo: "true", ballotData: "{}"})
    clearWorkbenchStorage(local, session)
    expect([...Array(local.length).keys()].map((index) => local.key(index))).toEqual(["unrelated"])
    expect(session.length).toBe(0)
})

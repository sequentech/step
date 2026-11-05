// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {describe, expect, test} from "vitest"
import {EOverVotePolicy} from "@sequentech/ui-core"
import {ScenarioId, scenarioSnapshot} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {PreviewScreen} from "voting-portal/src/preview/screens"
import {STATE_KEY, type StorageArea} from "./persistence"
import {
    ActionType,
    exportedSnapshot,
    initialState,
    screenPath,
    workbenchReducer,
    WorkbenchEventKind,
    type WorkbenchState,
} from "./state"

const storage = (stored?: string): StorageArea => ({
    length: stored === undefined ? 0 : 1,
    key: () => (stored === undefined ? null : STATE_KEY),
    getItem: (key) => (key === STATE_KEY ? (stored ?? null) : null),
    setItem: () => undefined,
    removeItem: () => undefined,
    clear: () => undefined,
})

const overVote = {
    [IDS.contest]: {over_vote_policy: EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_DISABLE},
}

const fresh = () => initialState(storage(), PreviewScreen.VOTE)

describe("the initial state", () => {
    test("opens the default scenario at the requested screen", () => {
        const state = fresh()
        expect(state.base).toEqual(scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY))
        expect(state.session).toEqual({snapshot: state.base, screen: PreviewScreen.VOTE})
        expect(state.events).toEqual([
            {id: 1, kind: WorkbenchEventKind.SESSION, message: "Opened the default scenario"},
        ])
    })

    test("restores the stored snapshot with its overrides applied", () => {
        const base = scenarioSnapshot(ScenarioId.KIOSK_VOTER)
        const state = initialState(
            storage(JSON.stringify({base, overrides: overVote})),
            PreviewScreen.CHOOSER
        )
        expect(state.base).toEqual(base)
        expect(state.overrides).toEqual(overVote)
        expect(state.session.screen).toBe(PreviewScreen.CHOOSER)
        expect(
            state.session.snapshot.preview.ballot_styles[0].contests[0].presentation
        ).toMatchObject({over_vote_policy: EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_DISABLE})
    })

    test("discards a stored state that no longer validates", () => {
        const state = initialState(storage('{"base": {"version": 0}}'), PreviewScreen.START)
        expect(state.base.scenarioId).toBe(ScenarioId.SIMPLE_PLURALITY)
        expect(state.events.map(({kind}) => kind)).toEqual([
            WorkbenchEventKind.STORAGE,
            WorkbenchEventKind.SESSION,
        ])
        expect(state.events[0].message).toContain("version: expected 1, found 0")
    })
})

describe("the reducer", () => {
    test("opening a snapshot replaces the session and the overrides", () => {
        const withOverrides = workbenchReducer(fresh(), {
            type: ActionType.OVERRIDE,
            overrides: overVote,
            screen: PreviewScreen.VOTE,
            message: "over_vote_policy",
        })
        const base = scenarioSnapshot(ScenarioId.RANKED_MULTI_CONTEST)
        const state = workbenchReducer(withOverrides, {
            type: ActionType.OPEN,
            base,
            overrides: {},
            screen: PreviewScreen.REVIEW,
            kind: WorkbenchEventKind.IMPORT,
            message: "Imported",
        })
        expect(state.base).toBe(base)
        expect(state.overrides).toEqual({})
        expect(state.session).toEqual({snapshot: base, screen: PreviewScreen.REVIEW})
        expect(state.events.at(-1)).toEqual({
            id: 3,
            kind: WorkbenchEventKind.IMPORT,
            message: "Imported",
        })
    })

    test("overrides start a new session for the current screen and clear import issues", () => {
        const rejected = workbenchReducer(fresh(), {
            type: ActionType.IMPORT_FAILED,
            issues: ["version: expected 1, found 2"],
        })
        expect(rejected.importIssues).toEqual(["version: expected 1, found 2"])
        const state = workbenchReducer(rejected, {
            type: ActionType.OVERRIDE,
            overrides: overVote,
            screen: PreviewScreen.START,
            message: "over_vote_policy",
        })
        expect(state.importIssues).toBeUndefined()
        expect(state.base).toBe(rejected.base)
        expect(state.session.screen).toBe(PreviewScreen.START)
        expect(state.session).not.toBe(rejected.session)
        expect(
            state.session.snapshot.preview.ballot_styles[0].contests[0].presentation
        ).toMatchObject({over_vote_policy: EOverVotePolicy.NOT_ALLOWED_WITH_MSG_AND_DISABLE})
    })

    test("the event log keeps the latest fifty entries", () => {
        let state: WorkbenchState = fresh()
        for (let index = 0; index < 60; index++)
            state = workbenchReducer(state, {
                type: ActionType.LOG,
                kind: WorkbenchEventKind.NETWORK,
                message: `request ${index}`,
            })
        expect(state.events).toHaveLength(50)
        expect(state.events[0]).toMatchObject({id: 12, message: "request 10"})
        expect(state.events.at(-1)).toMatchObject({id: 61, message: "request 59"})
    })
})

test("an export carries the overrides in its document and lists them as provenance", () => {
    const state = workbenchReducer(fresh(), {
        type: ActionType.OVERRIDE,
        overrides: overVote,
        screen: PreviewScreen.VOTE,
        message: "over_vote_policy",
    })
    const exported = exportedSnapshot(state, new Date("2026-02-01T10:00:00.000Z"))
    expect(exported.preview).toBe(state.session.snapshot.preview)
    expect(exported.provenance).toEqual({
        origin: "exported",
        createdAt: "2026-02-01T10:00:00.000Z",
        changes: ["Council representative: over_vote_policy = not-allowed-with-msg-and-disable"],
    })
    // A later export keeps the changes an imported snapshot already had.
    const reimported = workbenchReducer(state, {
        type: ActionType.OPEN,
        base: exported,
        overrides: {},
        screen: PreviewScreen.VOTE,
        kind: WorkbenchEventKind.IMPORT,
        message: "Imported",
    })
    expect(exportedSnapshot(reimported, new Date()).provenance.changes).toEqual(
        exported.provenance.changes
    )
})

test("screens without an election open the chooser", () => {
    const snapshot = scenarioSnapshot(ScenarioId.SIMPLE_PLURALITY)
    const event = `/tenant/${IDS.tenant}/event/${IDS.event}`
    expect(screenPath(snapshot, PreviewScreen.VOTE)).toBe(`${event}/election/${IDS.election}/vote`)
    expect(screenPath({...snapshot, areaId: "other"}, PreviewScreen.VOTE)).toBe(
        `${event}/election-chooser`
    )
})

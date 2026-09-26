// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useContext, useEffect, useMemo, useReducer} from "react"
import {useLocation, useNavigate} from "react-router"
import {
    parseSnapshot,
    scenarioSnapshot,
    serializeSnapshot,
    SnapshotError,
    SnapshotOrigin,
    ScenarioId,
    type ScenarioSnapshot,
} from "@sequentech/ui-test-kit/fixtures/scenarios"
import {
    PreviewScreen,
    previewScreenAt,
    previewScreenPath,
    previewTarget,
} from "voting-portal/src/preview/screens"
import type {PreviewSession} from "voting-portal/src/preview/session"
import {
    applyPolicyOverrides,
    describePolicyOverrides,
    withOverride,
    type BoundKey,
    type PolicyKey,
    type PolicyOverrides,
} from "./policies"
import {
    clearWorkbenchStorage,
    readPersistedState,
    writePersistedState,
    type StorageArea,
} from "./persistence"

export const DEFAULT_SCENARIO = ScenarioId.SIMPLE_PLURALITY
const EVENT_LIMIT = 50

export enum WorkbenchEventKind {
    SESSION = "session",
    POLICY = "policy",
    IMPORT = "import",
    EXPORT = "export",
    RESET = "reset",
    LOGOUT = "logout",
    NETWORK = "network",
    STORAGE = "storage",
}

export interface WorkbenchEvent {
    id: number
    kind: WorkbenchEventKind
    message: string
}

export interface WorkbenchState {
    /** The snapshot as selected or imported, without local overrides. */
    base: ScenarioSnapshot
    overrides: PolicyOverrides
    /** What the portal shows: the base with overrides applied, prepared for a screen. */
    session: PreviewSession
    events: readonly WorkbenchEvent[]
    importIssues?: string[]
}

export enum ActionType {
    OPEN = "open",
    OVERRIDE = "override",
    IMPORT_FAILED = "import-failed",
    LOG = "log",
}

export type WorkbenchAction =
    | {
          type: ActionType.OPEN
          base: ScenarioSnapshot
          overrides: PolicyOverrides
          screen: PreviewScreen
          kind: WorkbenchEventKind
          message: string
      }
    | {
          type: ActionType.OVERRIDE
          overrides: PolicyOverrides
          screen: PreviewScreen
          message: string
      }
    | {type: ActionType.IMPORT_FAILED; issues: string[]}
    | {type: ActionType.LOG; kind: WorkbenchEventKind; message: string}

const logged = (
    events: readonly WorkbenchEvent[],
    kind: WorkbenchEventKind,
    message: string
): WorkbenchEvent[] => [
    ...events.slice(1 - EVENT_LIMIT),
    {id: (events.at(-1)?.id ?? 0) + 1, kind, message},
]

const session = (base: ScenarioSnapshot, overrides: PolicyOverrides, screen: PreviewScreen) => ({
    snapshot: applyPolicyOverrides(base, overrides),
    screen,
})

export function workbenchReducer(state: WorkbenchState, action: WorkbenchAction): WorkbenchState {
    switch (action.type) {
        case ActionType.OPEN:
            return {
                base: action.base,
                overrides: action.overrides,
                session: session(action.base, action.overrides, action.screen),
                events: logged(state.events, action.kind, action.message),
            }
        case ActionType.OVERRIDE:
            return {
                ...state,
                overrides: action.overrides,
                session: session(state.base, action.overrides, action.screen),
                events: logged(state.events, WorkbenchEventKind.POLICY, action.message),
                importIssues: undefined,
            }
        case ActionType.IMPORT_FAILED:
            return {
                ...state,
                importIssues: action.issues,
                events: logged(state.events, WorkbenchEventKind.IMPORT, "Import rejected"),
            }
        case ActionType.LOG:
            return {...state, events: logged(state.events, action.kind, action.message)}
    }
}

/** The persisted state if it still validates, else a fresh default scenario. */
export function initialState(storage: StorageArea, screen: PreviewScreen): WorkbenchState {
    const opened = (
        base: ScenarioSnapshot,
        overrides: PolicyOverrides,
        events: WorkbenchEvent[]
    ): WorkbenchState => ({base, overrides, session: session(base, overrides, screen), events})
    const fresh = (events: WorkbenchEvent[]) =>
        opened(
            scenarioSnapshot(DEFAULT_SCENARIO),
            {},
            logged(events, WorkbenchEventKind.SESSION, "Opened the default scenario")
        )
    try {
        const persisted = readPersistedState(storage)
        if (!persisted) return fresh([])
        return opened(
            persisted.base,
            persisted.overrides,
            logged([], WorkbenchEventKind.SESSION, "Restored the stored snapshot")
        )
    } catch (error) {
        return fresh(
            logged([], WorkbenchEventKind.STORAGE, `Discarded the stored state: ${String(error)}`)
        )
    }
}

/** The production route of a screen, or the chooser when the area has no election. */
export function screenPath(snapshot: ScenarioSnapshot, screen: PreviewScreen) {
    const target = previewTarget(snapshot)
    return (
        previewScreenPath(target, screen) ??
        `/tenant/${target.tenantId}/event/${target.eventId}/election-chooser`
    )
}

/** Exported snapshots carry their overrides in the document and list them as provenance. */
export function exportedSnapshot(
    {base, overrides, session}: WorkbenchState,
    now: Date
): ScenarioSnapshot {
    return {
        ...session.snapshot,
        provenance: {
            origin: SnapshotOrigin.EXPORTED,
            createdAt: now.toISOString(),
            changes: [...base.provenance.changes, ...describePolicyOverrides(base, overrides)],
        },
    }
}

const WorkbenchContext = createContext<
    {state: WorkbenchState; dispatch: React.Dispatch<WorkbenchAction>} | undefined
>(undefined)

export const WorkbenchProvider: React.FC<React.PropsWithChildren<{initial: WorkbenchState}>> = ({
    initial,
    children,
}) => {
    const [state, dispatch] = useReducer(workbenchReducer, initial)
    useEffect(() => {
        writePersistedState(window.localStorage, {base: state.base, overrides: state.overrides})
    }, [state.base, state.overrides])
    const value = useMemo(() => ({state, dispatch}), [state])
    return <WorkbenchContext.Provider value={value}>{children}</WorkbenchContext.Provider>
}

export function useWorkbench() {
    const context = useContext(WorkbenchContext)
    if (!context) throw new Error("useWorkbench needs a WorkbenchProvider")
    return context
}

/** The screen shown at the current location, or where a new session should open. */
export function useCurrentScreen() {
    const {pathname} = useLocation()
    const {state} = useWorkbench()
    return previewScreenAt(pathname) ?? state.session.screen
}

/** Workbench commands: each opens its session and navigates to the screen's production route. */
export function useWorkbenchActions() {
    const {state, dispatch} = useWorkbench()
    const navigate = useNavigate()
    const screen = useCurrentScreen()

    return useMemo(() => {
        const go = (base: ScenarioSnapshot, target: PreviewScreen, replace = false) =>
            navigate(screenPath(base, target), {replace})
        const open = (
            base: ScenarioSnapshot,
            target: PreviewScreen,
            kind: WorkbenchEventKind,
            message: string,
            overrides: PolicyOverrides = {},
            replace = false
        ) => {
            dispatch({type: ActionType.OPEN, base, overrides, screen: target, kind, message})
            go(base, target, replace)
        }
        return {
            openScenario: (id: ScenarioId, target = screen, replace = false) =>
                open(
                    scenarioSnapshot(id),
                    target,
                    WorkbenchEventKind.SESSION,
                    `Opened ${id} at ${target}`,
                    {},
                    replace
                ),
            openScreen: (target: PreviewScreen) =>
                open(
                    state.base,
                    target,
                    WorkbenchEventKind.SESSION,
                    `Opened ${target}`,
                    state.overrides
                ),
            setOverride: (
                contestId: string,
                key: PolicyKey | BoundKey,
                value: string | number | undefined
            ) =>
                dispatch({
                    type: ActionType.OVERRIDE,
                    overrides: withOverride(state.overrides, contestId, key, value),
                    screen,
                    message: `${key} ${value === undefined ? "restored" : `= ${value}`}`,
                }),
            clearOverrides: () =>
                dispatch({
                    type: ActionType.OVERRIDE,
                    overrides: {},
                    screen,
                    message: "Cleared every override",
                }),
            importSnapshot: async (file: Blob) => {
                try {
                    const base = parseSnapshot(await file.text())
                    open(
                        base,
                        screen,
                        WorkbenchEventKind.IMPORT,
                        `Imported a ${base.provenance.origin} ${base.scenarioId} snapshot`
                    )
                } catch (error) {
                    dispatch({
                        type: ActionType.IMPORT_FAILED,
                        issues: error instanceof SnapshotError ? error.issues : [String(error)],
                    })
                }
            },
            exportSnapshot: () => {
                const snapshot = exportedSnapshot(state, new Date())
                const link = document.createElement("a")
                link.href = URL.createObjectURL(
                    new Blob([serializeSnapshot(snapshot)], {type: "application/json"})
                )
                link.download = `${snapshot.scenarioId}-snapshot.json`
                link.click()
                setTimeout(() => URL.revokeObjectURL(link.href))
                dispatch({
                    type: ActionType.LOG,
                    kind: WorkbenchEventKind.EXPORT,
                    message: `Exported ${link.download}`,
                })
            },
            reset: () => {
                clearWorkbenchStorage(window.localStorage, window.sessionStorage)
                open(
                    scenarioSnapshot(state.base.scenarioId),
                    screen,
                    WorkbenchEventKind.RESET,
                    `Reset to the bundled ${state.base.scenarioId} scenario`
                )
            },
        }
    }, [state, dispatch, navigate, screen])
}

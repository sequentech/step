// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {setIn} from "@/components/monitoring/lib/yamlPatch"
import {EDiagnosticOrigin} from "@/components/monitoring/lib/diagnostics"
import {
    EFormAccess,
    EPreviewStatus,
    PREVIEW_DEBOUNCE_MS,
    YamlDraftController,
    type TRenderPreview,
} from "./yamlDraft"
import {
    EMonitoringProblemSeverity,
    EMonitoringRenderState,
    type IMonitoringRenderResponse,
} from "./types"

const TEXT = "id: turnout\ntitle: Turnout # card title\n"

const rendered = (render_ms: number): IMonitoringRenderResponse => ({
    state: EMonitoringRenderState.RENDERED,
    svg: `<svg data-ms="${render_ms}"/>`,
    render_ms,
    diagnostics: [],
})

/** A render whose replies the test releases one by one. */
const controlledRender = () => {
    const pending: Array<{text: string; resolve: (value: IMonitoringRenderResponse) => void}> = []
    const render: TRenderPreview = (text) =>
        new Promise((resolve) => {
            pending.push({text, resolve})
        })
    return {render, pending}
}

const flush = () => new Promise((resolve) => jest.requireActual("timers").setImmediate(resolve))

beforeEach(() => {
    jest.useFakeTimers()
})

afterEach(() => {
    jest.useRealTimers()
})

describe("YamlDraftController", () => {
    it("forms patch the text, which stays the only copy", () => {
        const draft = new YamlDraftController({text: TEXT})
        expect(draft.patch((text) => setIn(text, ["title"], "Voter turnout"))).toBe(true)
        expect(draft.getState().text).toBe("id: turnout\ntitle: Voter turnout # card title\n")
        expect(draft.getState().value).toEqual({id: "turnout", title: "Voter turnout"})
        expect(draft.getState().dirty).toBe(true)
    })

    it("makes the forms read-only while the YAML has a syntax error", () => {
        const draft = new YamlDraftController({text: TEXT})
        draft.setText("id: [turnout\n")
        const state = draft.getState()
        expect(state.formAccess).toBe(EFormAccess.READ_ONLY)
        expect(state.value).toBeUndefined()
        expect(state.diagnostics[0].origin).toBe(EDiagnosticOrigin.SYNTAX)
        expect(draft.patch((text) => setIn(text, ["title"], "x"))).toBe(false)
        expect(draft.getState().text).toBe("id: [turnout\n")
    })

    it("runs the local checks on every change", () => {
        const localValidate = jest.fn((text: string) =>
            text.includes("sql")
                ? [{severity: "error", code: "forbidden_key", path: "sql", message: "No SQL"}]
                : []
        )
        const draft = new YamlDraftController({
            text: TEXT,
            localValidate: localValidate as never,
        })
        draft.setText(`${TEXT}sql: select 1\n`)
        expect(draft.getState().localProblems).toEqual([
            expect.objectContaining({
                severity: EMonitoringProblemSeverity.ERROR,
                code: "forbidden_key",
            }),
        ])
        expect(draft.getState().diagnostics[0]).toEqual(
            expect.objectContaining({origin: EDiagnosticOrigin.LOCAL, line: 3})
        )
        draft.setText(TEXT)
        expect(draft.getState().localProblems).toEqual([])
        expect(localValidate).toHaveBeenCalledTimes(3)
    })

    it("falls back to server checks when the local ones are unavailable or fail", () => {
        const unavailable = new YamlDraftController({text: TEXT, localValidate: () => null})
        expect(unavailable.getState().localUnavailable).toBe(true)
        const throwing = new YamlDraftController({
            text: TEXT,
            localValidate: () => {
                throw new Error("wasm not loaded")
            },
        })
        expect(throwing.getState().localUnavailable).toBe(true)
        expect(new YamlDraftController({text: TEXT}).getState().localUnavailable).toBe(true)
    })

    it("renders the preview once per pause in typing", async () => {
        const render = jest.fn(async () => rendered(12))
        const draft = new YamlDraftController({text: TEXT, renderPreview: render})
        draft.setText(`${TEXT}height: 3\n`)
        jest.advanceTimersByTime(PREVIEW_DEBOUNCE_MS - 1)
        draft.setText(`${TEXT}height: 30\n`)
        jest.advanceTimersByTime(PREVIEW_DEBOUNCE_MS - 1)
        draft.setText(`${TEXT}height: 300\n`)
        expect(render).not.toHaveBeenCalled()
        jest.advanceTimersByTime(PREVIEW_DEBOUNCE_MS)
        expect(render).toHaveBeenCalledTimes(1)
        expect(render).toHaveBeenCalledWith(`${TEXT}height: 300\n`)
        await flush()
        expect(draft.getState().previewStatus).toBe(EPreviewStatus.READY)
        expect(draft.getState().preview?.render_ms).toBe(12)
    })

    it("discards a reply that arrives after a newer request was sent", async () => {
        const {render, pending} = controlledRender()
        const draft = new YamlDraftController({text: TEXT, renderPreview: render})
        draft.setText(`${TEXT}height: 1\n`)
        jest.advanceTimersByTime(PREVIEW_DEBOUNCE_MS)
        draft.setText(`${TEXT}height: 2\n`)
        jest.advanceTimersByTime(PREVIEW_DEBOUNCE_MS)
        expect(pending).toHaveLength(2)
        pending[1].resolve(rendered(2))
        await flush()
        pending[0].resolve(rendered(1))
        await flush()
        expect(draft.getState().preview?.render_ms).toBe(2)
    })

    it("does not render text that does not parse", () => {
        const render = jest.fn(async () => rendered(1))
        const draft = new YamlDraftController({text: TEXT, renderPreview: render})
        draft.setText("id: [\n")
        jest.advanceTimersByTime(PREVIEW_DEBOUNCE_MS)
        expect(render).not.toHaveBeenCalled()
    })

    it("shows the server's problems from the preview beside the local ones", async () => {
        const render = jest.fn(async () => ({
            ...rendered(5),
            state: EMonitoringRenderState.INVALID,
            diagnostics: [
                {
                    severity: "ERROR",
                    code: "chart_schema",
                    path: "title",
                    message: "dbt Charts refused it",
                    engine_code: "ERR-001",
                },
            ],
        }))
        const draft = new YamlDraftController({text: TEXT, renderPreview: render as never})
        await draft.previewNow()
        expect(draft.getState().diagnostics).toEqual([
            expect.objectContaining({
                origin: EDiagnosticOrigin.SERVER,
                engine_code: "ERR-001",
                line: 2,
            }),
        ])
    })

    it("reports a failed render without losing the last preview", async () => {
        let fail = false
        const draft = new YamlDraftController({
            text: TEXT,
            renderPreview: async () => {
                if (fail) throw new Error("renderer unavailable")
                return rendered(3)
            },
        })
        await draft.previewNow()
        fail = true
        await draft.previewNow()
        expect(draft.getState().previewStatus).toBe(EPreviewStatus.FAILED)
        expect(draft.getState().previewError).toBe("renderer unavailable")
        expect(draft.getState().preview?.render_ms).toBe(3)
    })

    it("a reset makes the draft clean and drops the server's old problems", () => {
        const draft = new YamlDraftController({text: TEXT})
        draft.setText(`${TEXT}height: 1\n`)
        draft.setServerProblems([{severity: "ERROR", code: "x", path: "", message: "old"}])
        draft.reset("id: other\n")
        expect(draft.getState()).toEqual(
            expect.objectContaining({dirty: false, text: "id: other\n", serverProblems: []})
        )
    })

    it("notifies subscribers until they unsubscribe", () => {
        const draft = new YamlDraftController({text: TEXT})
        const listener = jest.fn()
        const unsubscribe = draft.subscribe(listener)
        draft.setText(`${TEXT}# more\n`)
        expect(listener).toHaveBeenCalledTimes(1)
        unsubscribe()
        draft.setText(TEXT)
        expect(listener).toHaveBeenCalledTimes(1)
    })
})

describe("YamlDraftController.dispose", () => {
    beforeEach(() => {
        jest.useFakeTimers()
    })

    afterEach(() => {
        jest.useRealTimers()
    })

    it("cancels the pending render and ignores a reply in flight", async () => {
        const {render, pending} = controlledRender()
        const draft = new YamlDraftController({text: TEXT, renderPreview: render})
        draft.setText(`${TEXT}height: 1\n`)
        jest.advanceTimersByTime(PREVIEW_DEBOUNCE_MS)
        draft.setText(`${TEXT}height: 2\n`)
        draft.dispose()
        jest.advanceTimersByTime(PREVIEW_DEBOUNCE_MS)
        expect(pending).toHaveLength(1)
        pending[0].resolve(rendered(1))
        await flush()
        expect(draft.getState().preview).toBeUndefined()
    })
})

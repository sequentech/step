// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The state of one document being edited, outside React so it can be tested
 * with plain timers.
 *
 * - The YAML text is the single source of truth. Forms patch it; they never
 *   keep values of their own.
 * - While the text does not parse, forms are read-only: a patch would have
 *   to guess what the author meant.
 * - Local checks (sequent-core in the browser) run on every change.
 * - The live preview is rendered at most once per pause in typing, and a
 *   reply that arrives after a newer request was sent is thrown away.
 */

import {
    editorDiagnostics,
    normalizeProblems,
    type IEditorDiagnostic,
} from "@/components/monitoring/lib/diagnostics"
import {
    EYamlParseStatus,
    parseYamlText,
    type TYamlParseResult,
} from "@/components/monitoring/lib/yamlPatch"
import type {IMonitoringProblem, IMonitoringRenderResponse} from "./types"

export const PREVIEW_DEBOUNCE_MS = 400

/** Checks the text in the browser; `null` when no local checks are available. */
export type TLocalValidate = (text: string) => IMonitoringProblem[] | null
export type TRenderPreview = (text: string) => Promise<IMonitoringRenderResponse>

export enum EPreviewStatus {
    IDLE = "IDLE",
    RENDERING = "RENDERING",
    READY = "READY",
    FAILED = "FAILED",
}

export enum EFormAccess {
    EDITABLE = "EDITABLE",
    /** The YAML has a syntax error; fix it in the YAML tab first. */
    READ_ONLY = "READ_ONLY",
}

export interface IYamlDraftState {
    text: string
    /** The text as last loaded or saved. */
    baseline: string
    dirty: boolean
    parsed: TYamlParseResult
    /** The document as data, or `undefined` while it does not parse. */
    value: unknown
    formAccess: EFormAccess
    localProblems: IMonitoringProblem[]
    /** `true` when no local checks run, so only the server's are shown. */
    localUnavailable: boolean
    serverProblems: IMonitoringProblem[]
    diagnostics: IEditorDiagnostic[]
    previewStatus: EPreviewStatus
    preview?: IMonitoringRenderResponse
    previewError?: string
}

export interface IYamlDraftOptions {
    text: string
    localValidate?: TLocalValidate
    renderPreview?: TRenderPreview
    debounceMs?: number
}

type TListener = () => void

export class YamlDraftController {
    private state: IYamlDraftState
    private readonly listeners = new Set<TListener>()
    private timer: ReturnType<typeof setTimeout> | undefined
    private issued = 0

    constructor(private options: IYamlDraftOptions) {
        this.state = this.derive(options.text, options.text, [], {
            previewStatus: EPreviewStatus.IDLE,
        })
    }

    /** Swaps the callbacks (they close over props that change) without resetting the draft. */
    configure(options: Partial<Omit<IYamlDraftOptions, "text">>) {
        this.options = {...this.options, ...options}
    }

    subscribe = (listener: TListener) => {
        this.listeners.add(listener)
        return () => {
            this.listeners.delete(listener)
        }
    }

    getState = () => this.state

    setText(text: string) {
        if (text === this.state.text) return
        this.state = this.derive(text, this.state.baseline, this.state.serverProblems, this.state)
        this.emit()
        this.schedulePreview()
    }

    /**
     * Applies a form's change to the text. Refused (and `false`) while the text
     * does not parse, or when the change throws.
     */
    patch(change: (text: string) => string): boolean {
        if (this.state.formAccess !== EFormAccess.EDITABLE) return false
        let next: string
        try {
            next = change(this.state.text)
        } catch {
            return false
        }
        this.setText(next)
        return true
    }

    /** A new starting point, after loading or saving: the draft is clean again. */
    reset(text: string) {
        this.clearTimer()
        this.issued += 1
        this.state = this.derive(text, text, [], this.state)
        this.emit()
        this.schedulePreview()
    }

    /** Marks the current text as saved without replacing it. */
    markSaved() {
        this.state = {...this.state, baseline: this.state.text, dirty: false}
        this.emit()
    }

    setServerProblems(problems: unknown) {
        this.state = this.derive(
            this.state.text,
            this.state.baseline,
            normalizeProblems(problems),
            this.state
        )
        this.emit()
    }

    /** Takes a preview from elsewhere (Validate returns one), as the newest. */
    acceptPreview(preview: IMonitoringRenderResponse) {
        this.issued += 1
        this.clearTimer()
        this.applyPreview(preview)
    }

    /** Renders now instead of after the pause. */
    previewNow(): Promise<void> {
        this.clearTimer()
        return this.render()
    }

    /** Cancels the pending render and discards any reply still in flight. */
    dispose() {
        this.clearTimer()
        this.issued += 1
    }

    private schedulePreview() {
        if (!this.options.renderPreview) return
        this.clearTimer()
        this.timer = setTimeout(() => {
            this.timer = undefined
            void this.render()
        }, this.options.debounceMs ?? PREVIEW_DEBOUNCE_MS)
    }

    private async render(): Promise<void> {
        const renderPreview = this.options.renderPreview
        if (!renderPreview || this.state.parsed.status !== EYamlParseStatus.OK) return
        this.issued += 1
        const request = this.issued
        this.state = {...this.state, previewStatus: EPreviewStatus.RENDERING}
        this.emit()
        try {
            const preview = await renderPreview(this.state.text)
            if (request !== this.issued) return
            this.applyPreview(preview)
        } catch (error) {
            if (request !== this.issued) return
            this.state = {
                ...this.state,
                previewStatus: EPreviewStatus.FAILED,
                previewError: error instanceof Error ? error.message : String(error),
            }
            this.emit()
        }
    }

    private applyPreview(preview: IMonitoringRenderResponse) {
        this.state = {
            ...this.derive(
                this.state.text,
                this.state.baseline,
                normalizeProblems(preview.diagnostics ?? []),
                this.state
            ),
            previewStatus: EPreviewStatus.READY,
            preview,
            previewError: undefined,
        }
        this.emit()
    }

    private derive(
        text: string,
        baseline: string,
        serverProblems: IMonitoringProblem[],
        previous: Pick<IYamlDraftState, "previewStatus" | "preview" | "previewError">
    ): IYamlDraftState {
        const parsed = parseYamlText(text)
        const ok = parsed.status === EYamlParseStatus.OK
        const local = ok ? this.localProblems(text) : []
        const localProblems = local ?? []
        return {
            text,
            baseline,
            dirty: text !== baseline,
            parsed,
            value: ok ? parsed.value : undefined,
            formAccess: ok ? EFormAccess.EDITABLE : EFormAccess.READ_ONLY,
            localProblems,
            localUnavailable: local === null,
            serverProblems,
            diagnostics: editorDiagnostics(text, parsed, {
                local: localProblems,
                server: serverProblems,
            }),
            previewStatus: previous.previewStatus,
            preview: previous.preview,
            previewError: previous.previewError,
        }
    }

    private localProblems(text: string): IMonitoringProblem[] | null {
        const validate = this.options.localValidate
        if (!validate) return null
        try {
            const problems = validate(text)
            return problems === null ? null : normalizeProblems(problems)
        } catch {
            return null
        }
    }

    private clearTimer() {
        if (this.timer !== undefined) {
            clearTimeout(this.timer)
            this.timer = undefined
        }
    }

    private emit() {
        Array.from(this.listeners).forEach((listener) => listener())
    }
}

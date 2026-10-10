// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useImperativeHandle, useRef, type Ref} from "react"
import {Box} from "@mui/material"
import {EditorState, type Extension} from "@codemirror/state"
import {
    EditorView,
    drawSelection,
    highlightActiveLine,
    highlightActiveLineGutter,
    keymap,
    lineNumbers,
} from "@codemirror/view"
import {defaultKeymap, history, historyKeymap} from "@codemirror/commands"
import {
    bracketMatching,
    defaultHighlightStyle,
    indentOnInput,
    syntaxHighlighting,
} from "@codemirror/language"
import {yaml} from "@codemirror/lang-yaml"
import {lintGutter, setDiagnostics, type Diagnostic} from "@codemirror/lint"
import {EMonitoringProblemSeverity} from "./types"
import type {IEditorDiagnostic} from "@/components/monitoring/lib/diagnostics"

export interface IMonitoringYamlEditorHandle {
    /** Selects `from`–`to` and scrolls it into view, for a problem picked from the list. */
    reveal(from: number, to: number): void
}

export interface MonitoringYamlEditorProps {
    value: string
    onChange: (value: string) => void
    diagnostics?: IEditorDiagnostic[]
    /** The accessible name of the text area. */
    label: string
    readOnly?: boolean
    minHeight?: number
    ref?: Ref<IMonitoringYamlEditorHandle>
}

const toCodeMirror = (text: string, diagnostics: IEditorDiagnostic[]): Diagnostic[] =>
    diagnostics.map((diagnostic) => {
        const from = Math.min(diagnostic.from, text.length)
        return {
            from,
            to: Math.min(Math.max(diagnostic.to, from), text.length),
            severity:
                diagnostic.severity === EMonitoringProblemSeverity.WARNING ? "warning" : "error",
            message: diagnostic.path
                ? `${diagnostic.path}: ${diagnostic.message}`
                : diagnostic.message,
            source: diagnostic.engine_code ?? undefined,
        }
    })

const theme = (minHeight: number) =>
    EditorView.theme({
        "&": {
            fontSize: "13px",
            border: "1px solid rgba(0, 0, 0, 0.23)",
            borderRadius: "4px",
            backgroundColor: "#fff",
        },
        "&.cm-focused": {outline: "2px solid #0f054c", outlineOffset: "-1px"},
        ".cm-scroller": {
            fontFamily: "'Roboto Mono', ui-monospace, SFMono-Regular, Menlo, monospace",
            minHeight: `${minHeight}px`,
        },
        ".cm-content": {minHeight: `${minHeight}px`},
    })

/** The whole YAML of a monitoring document, highlighted, with its problems inline. */
export const MonitoringYamlEditor: React.FC<MonitoringYamlEditorProps> = ({
    value,
    onChange,
    diagnostics = [],
    label,
    readOnly = false,
    minHeight = 320,
    ref,
}) => {
    const host = useRef<HTMLDivElement>(null)
    const view = useRef<EditorView | null>(null)
    const onChangeRef = useRef(onChange)
    onChangeRef.current = onChange

    useEffect(() => {
        if (!host.current) return
        const extensions: Extension[] = [
            lineNumbers(),
            highlightActiveLineGutter(),
            history(),
            drawSelection(),
            indentOnInput(),
            bracketMatching(),
            highlightActiveLine(),
            syntaxHighlighting(defaultHighlightStyle, {fallback: true}),
            yaml(),
            lintGutter(),
            keymap.of([...defaultKeymap, ...historyKeymap]),
            EditorState.tabSize.of(2),
            EditorView.lineWrapping,
            EditorState.readOnly.of(readOnly),
            EditorView.editable.of(!readOnly),
            EditorView.contentAttributes.of({
                "aria-label": label,
                "aria-multiline": "true",
                "role": "textbox",
                "data-testid": "monitoring-yaml-editor",
            }),
            EditorView.updateListener.of((update) => {
                if (update.docChanged) onChangeRef.current(update.state.doc.toString())
            }),
            theme(minHeight),
        ]
        const editor = new EditorView({
            parent: host.current,
            state: EditorState.create({doc: value, extensions}),
        })
        view.current = editor
        return () => {
            editor.destroy()
            view.current = null
        }
        // The editor is built once per configuration; text changes are dispatched below.
    }, [label, readOnly, minHeight])

    useEffect(() => {
        const editor = view.current
        if (!editor) return
        const current = editor.state.doc.toString()
        if (current !== value) {
            editor.dispatch({changes: {from: 0, to: current.length, insert: value}})
        }
    }, [value])

    useEffect(() => {
        const editor = view.current
        if (!editor) return
        const text = editor.state.doc.toString()
        editor.dispatch(setDiagnostics(editor.state, toCodeMirror(text, diagnostics)))
    }, [diagnostics, value])

    useImperativeHandle(
        ref,
        () => ({
            reveal(from, to) {
                const editor = view.current
                if (!editor) return
                const length = editor.state.doc.length
                const anchor = Math.min(from, length)
                editor.dispatch({
                    selection: {anchor, head: Math.min(Math.max(to, anchor), length)},
                    scrollIntoView: true,
                })
                editor.focus()
            },
        }),
        []
    )

    return <Box ref={host} className="monitoring-yaml-editor" sx={{width: "100%"}} />
}

export default MonitoringYamlEditor

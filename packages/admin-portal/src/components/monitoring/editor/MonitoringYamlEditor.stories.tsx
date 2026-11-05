// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {editorDiagnostics} from "@/components/monitoring/lib/diagnostics"
import {parseYamlText} from "@/components/monitoring/lib/yamlPatch"
import {MonitoringYamlEditor, type MonitoringYamlEditorProps} from "./MonitoringYamlEditor"
import {FORBIDDEN_KEY, WIDGET_YAML} from "./storyFixtures"

/** Keeps the text in state, as the configure dialog's draft does. */
const Controlled = (props: MonitoringYamlEditorProps) => {
    const [value, setValue] = useState(props.value)
    return (
        <MonitoringYamlEditor
            {...props}
            value={value}
            onChange={(next) => {
                setValue(next)
                props.onChange(next)
            }}
        />
    )
}

const meta = {
    title: "Admin/Monitoring/Editor/MonitoringYamlEditor",
    component: MonitoringYamlEditor,
    args: {value: WIDGET_YAML, onChange: fn(), label: "Widget YAML", minHeight: 200},
    render: (args) => <Controlled {...args} />,
} satisfies Meta<typeof MonitoringYamlEditor>
export default meta
type Story = StoryObj<typeof meta>

const editor = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("textbox", {name: "Widget YAML"})

export const Editing: Story = {
    play: async ({canvasElement, args}) => {
        const text = await editor(canvasElement)
        await expect(text).toHaveTextContent("id: turnout-by-group")
        await userEvent.click(text)
        await userEvent.keyboard("{Control>}{End}{/Control}# edited")
        await waitFor(() =>
            expect(args.onChange).toHaveBeenLastCalledWith(expect.stringMatching(/# edited$/))
        )
    },
}

const withSql = `${WIDGET_YAML.replace("  template: by_group\n", "  template: by_group\n  sql: select 1\n")}`

export const WithProblems: Story = {
    args: {
        value: withSql,
        diagnostics: editorDiagnostics(withSql, parseYamlText(withSql), {
            local: [FORBIDDEN_KEY],
            server: [],
        }),
    },
    play: async ({canvasElement}) => {
        await editor(canvasElement)
        await waitFor(() =>
            expect(canvasElement.querySelector(".cm-lintRange-error")).toHaveTextContent(
                "sql: select 1"
            )
        )
        expect(canvasElement.querySelector(".cm-lint-marker-error")).not.toBeNull()
    },
}

export const ReadOnly: Story = {
    args: {readOnly: true},
    play: async ({canvasElement, args}) => {
        const text = await editor(canvasElement)
        await expect(text).toHaveAttribute("contenteditable", "false")
        expect(args.onChange).not.toHaveBeenCalled()
    },
}

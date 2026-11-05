// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {MonitoringSelectorsForm} from "./MonitoringSelectorsForm"
import {EYamlParseStatus, parseYamlText} from "@/components/monitoring/lib/yamlPatch"
import {EFormAccess} from "./yamlDraft"
import {WIDGET_YAML} from "./storyFixtures"

/** The form with a YAML text of its own, shown below it. */
interface IFormHarnessProps {
    yaml: string
    children: (props: {
        value: unknown
        formAccess: EFormAccess
        onPatch: (change: (text: string) => string) => void
    }) => React.ReactNode
}

const FormHarness: React.FC<IFormHarnessProps> = ({yaml, children}) => {
    const [text, setText] = useState(yaml)
    const parsed = parseYamlText(text)
    const ok = parsed.status === EYamlParseStatus.OK
    return (
        <>
            {children({
                value: ok ? parsed.value : undefined,
                formAccess: ok ? EFormAccess.EDITABLE : EFormAccess.READ_ONLY,
                onPatch: (change) => setText(change),
            })}
            <pre aria-label="Resulting YAML" style={{fontSize: 12}}>
                {text}
            </pre>
        </>
    )
}

const meta = {
    title: "Admin/Monitoring/Editor/MonitoringSelectorsForm",
    component: MonitoringSelectorsForm,
    args: {value: {}, formAccess: EFormAccess.EDITABLE, onPatch: () => undefined},
    render: () => (
        <FormHarness yaml={WIDGET_YAML}>
            {(props) => <MonitoringSelectorsForm {...props} />}
        </FormHarness>
    ),
} satisfies Meta<typeof MonitoringSelectorsForm>
export default meta
type Story = StoryObj<typeof meta>

const resulting = (canvasElement: HTMLElement) =>
    within(canvasElement).getByLabelText("Resulting YAML")

export const EditSelectors: Story = {
    parameters: {widgets: ["KeyField", "SelectorCard"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const breakdown = canvas.getByRole("region", {name: "Breakdown"})
        const label = within(breakdown).getByRole("textbox", {name: "Label"})
        await userEvent.clear(label)
        await userEvent.type(label, "Group")
        await expect(resulting(canvasElement)).toHaveTextContent("label: Group")
        // Comments elsewhere in the document survive a form edit.
        await expect(resulting(canvasElement)).toHaveTextContent("# One turnout ratio")

        await userEvent.click(canvas.getByRole("button", {name: "Move measure up"}))
        const text = resulting(canvasElement).textContent ?? ""
        expect(text.indexOf("measure:")).toBeLessThan(text.indexOf("breakdown:"))

        await userEvent.click(canvas.getByRole("button", {name: "Add selector"}))
        await expect(resulting(canvasElement)).toHaveTextContent("selector_1:")
        await userEvent.click(canvas.getByRole("button", {name: "Remove selector selector_1"}))
        expect(resulting(canvasElement)).not.toHaveTextContent("selector_1:")
    },
}

export const RenameKeepsDefault: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const breakdown = canvas.getByRole("region", {name: "Breakdown"})
        const values = within(breakdown).getAllByRole("textbox", {name: "Value"})
        const age = values.find((input) => (input as HTMLInputElement).value === "age_band")
        expect(age).toBeDefined()
        await userEvent.clear(age as HTMLElement)
        await userEvent.type(age as HTMLElement, "age")
        await userEvent.tab()
        await expect(resulting(canvasElement)).toHaveTextContent("default: age")
        await expect(resulting(canvasElement)).toHaveTextContent("age: Age")
    },
}

export const ReadOnlyOnSyntaxError: Story = {
    render: () => (
        <FormHarness yaml={"selectors: [unclosed"}>
            {(props) => <MonitoringSelectorsForm {...props} />}
        </FormHarness>
    ),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(/syntax error/)).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Add selector"})).toBeDisabled()
    },
}

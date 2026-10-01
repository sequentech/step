// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {MonitoringDataQueryForm} from "./MonitoringDataQueryForm"
import {EYamlParseStatus, parseYamlText} from "@/components/monitoring/lib/yamlPatch"
import {EFormAccess} from "./yamlDraft"
import {RENDERED, SOURCES, SUMMARY_YAML, WIDGET_YAML} from "./storyFixtures"

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
    title: "Admin/Monitoring/Editor/MonitoringDataQueryForm",
    component: MonitoringDataQueryForm,
    args: {
        value: {},
        formAccess: EFormAccess.EDITABLE,
        sources: SOURCES,
        onPatch: () => undefined,
        table: RENDERED.table,
    },
    render: (args) => (
        <FormHarness yaml={SUMMARY_YAML}>
            {(props) => <MonitoringDataQueryForm {...args} {...props} />}
        </FormHarness>
    ),
} satisfies Meta<typeof MonitoringDataQueryForm>
export default meta
type Story = StoryObj<typeof meta>

const resulting = (canvasElement: HTMLElement) =>
    within(canvasElement).getByLabelText("Resulting YAML")

const choose = async (canvasElement: HTMLElement, field: string, option: string) => {
    await userEvent.click(within(canvasElement).getByRole("combobox", {name: field}))
    await userEvent.click(
        await within(canvasElement.ownerDocument.body).findByRole("option", {name: option})
    )
}

export const EditQuery: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(/Counts distinct voters/)).toBeVisible()

        await choose(canvasElement, "Group by", "Region")
        await expect(resulting(canvasElement)).toHaveTextContent("group_by: region")

        await choose(canvasElement, "Sort", "Value")
        await expect(resulting(canvasElement)).toHaveTextContent(/sort:\s+by: value\s+order: desc/)

        const limit = canvas.getByRole("spinbutton", {name: "Rows"})
        await userEvent.type(limit, "10")
        await expect(resulting(canvasElement)).toHaveTextContent("limit: 10")

        await userEvent.click(canvas.getByRole("checkbox", {name: "Country"}))
        await expect(resulting(canvasElement)).toHaveTextContent("follows:")
        await userEvent.click(canvas.getByRole("checkbox", {name: "Country"}))
        expect(resulting(canvasElement)).not.toHaveTextContent("follows:")

        const table = canvas.getByRole("table", {name: "Query result · first rows"})
        await expect(within(table).getByText("18–29")).toBeVisible()
    },
}

export const FromSelectors: Story = {
    render: (args) => (
        <FormHarness yaml={WIDGET_YAML}>
            {(props) => <MonitoringDataQueryForm {...args} {...props} />}
        </FormHarness>
    ),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByDisplayValue("From selector: breakdown")).toBeDisabled()
        await expect(canvas.getByDisplayValue("From selector: measure")).toBeDisabled()
    },
}

export const NotConnected: Story = {
    render: (args) => (
        <FormHarness
            yaml={
                "id: attacks\ntitle: Attacks\nsource: attack_detections\nquery:\n  template: summary\n"
            }
        >
            {(props) => <MonitoringDataQueryForm {...args} {...props} table={null} />}
        </FormHarness>
    ),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(/Not connected · VOTE-SECOPS/)).toBeVisible()
        await expect(canvas.getByText(/Counts detections/)).toBeVisible()
    },
}

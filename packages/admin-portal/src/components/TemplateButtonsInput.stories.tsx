// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ETemplateButtonType, ITemplateButton} from "@/types/messaging"
import {TemplateButtonsInput} from "./TemplateButtonsInput"

const DEFAULTS: ITemplateButton[] = [
    {type: ETemplateButtonType.URL, parameter: "{{code}}"},
    {type: ETemplateButtonType.QUICK_REPLY, parameter: "NA"},
]

interface Scenario {
    initial: ITemplateButton[] | null
    disabled: boolean
}

/** Keeps the buttons the control reports, as a binding editor does. */
const Stateful: React.FC<Scenario> = ({initial, disabled}) => {
    const [buttons, setButtons] = useState<ITemplateButton[] | null>(initial)
    return (
        <TemplateButtonsInput
            buttons={buttons}
            defaults={DEFAULTS}
            disabled={disabled}
            onChange={setButtons}
        />
    )
}

const meta = {
    title: "Admin/Components/TemplateButtonsInput",
    component: TemplateButtonsInput,
    args: {initial: null, disabled: false},
    render: (args) => <Stateful {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const values = (canvasElement: HTMLElement) =>
    within(canvasElement)
        .queryAllByRole("textbox", {name: "Value"})
        .map((input) => (input as HTMLInputElement).value)

/** The provider's defaults for a code, then the template's own, edited and reordered by hand. */
export const DefaultThenOwn: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("radio", {name: "Default for the purpose"})).toBeChecked()
        await expect(
            canvas.getByText("URL (copy code or link): {{code}} · Quick reply: NA")
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("radio", {name: "Set for this template"}))
        await expect(values(canvasElement)).toEqual(["{{code}}", "NA"])
        const quickReply = canvas.getAllByRole("textbox", {name: "Value"})[1]
        await userEvent.clear(quickReply)
        await expect(canvas.getByText("Enter a value")).toBeVisible()
        await expect(quickReply).toHaveAccessibleDescription("Enter a value")
        await userEvent.type(quickReply, "SKIP")
        await userEvent.click(canvas.getAllByRole("button", {name: "Remove button"})[0])
        await expect(values(canvasElement)).toEqual(["SKIP"])
        await userEvent.click(canvas.getByRole("radio", {name: "Default for the purpose"}))
        await expect(values(canvasElement)).toEqual([])
    },
}

/** The rows fit their container and align each remove button with its value field. */
export const Responsive: Story = {
    args: {initial: [DEFAULTS[0], {type: ETemplateButtonType.QUICK_REPLY, parameter: ""}]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const valueInputs = canvas.getAllByRole("textbox", {name: "Value"})
        const removeButtons = canvas.getAllByRole("button", {name: "Remove button"})
        const control = canvasElement.querySelector(".MuiFormControl-root") as HTMLElement
        const originalWidth = control.style.width
        try {
            for (const width of [300, 520]) {
                control.style.width = `${width}px`
                await waitFor(() => {
                    const bounds = control.getBoundingClientRect()
                    for (const field of Array.from(
                        control.querySelectorAll(".MuiTextField-root")
                    )) {
                        const rect = field.getBoundingClientRect()
                        expect(rect.width).toBeGreaterThanOrEqual(100)
                        expect(rect.left).toBeGreaterThanOrEqual(bounds.left)
                        expect(rect.right).toBeLessThanOrEqual(bounds.right + 1)
                    }
                    valueInputs.forEach((input, index) => {
                        const value = input.getBoundingClientRect()
                        const remove = removeButtons[index].getBoundingClientRect()
                        const valueCenter = value.top + value.height / 2
                        const removeCenter = remove.top + remove.height / 2
                        expect(Math.abs(removeCenter - valueCenter)).toBeLessThanOrEqual(1)
                    })
                })
            }
        } finally {
            control.style.width = originalWidth
        }
    },
}

/** Read-only: the template's buttons, nothing to change. */
export const ReadOnly: Story = {
    args: {initial: DEFAULTS, disabled: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        expect(canvas.queryByRole("button", {name: "Add button"})).toBeNull()
        await expect(canvas.getAllByRole("textbox", {name: "Value"})[0]).toBeDisabled()
    },
}

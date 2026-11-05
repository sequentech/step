// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {TextField} from "@mui/material"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {VoterField} from "./VoterEditorLayout"

interface Scenario {
    name: string
    inputType: string
    required: boolean
}

const meta = {
    title: "Admin/User/VoterField",
    component: VoterField,
    args: {name: "city", inputType: "text", required: false},
    render: (args) => (
        <VoterField {...args}>
            <TextField label="City" required={args.required} defaultValue="Springfield" />
        </VoterField>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const wrapper = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("textbox", {name: /City/}).closest(".voter-field")

export const OptionalTextField: Story = {
    play: async ({canvasElement}) => {
        const field = wrapper(canvasElement)
        expect(field).toHaveAttribute("data-field-name", "city")
        expect(field).toHaveAttribute("data-input-type", "text")
        expect(field).toHaveAttribute("data-required", "false")
    },
}

export const RequiredDateField: Story = {
    args: {name: "date-of-birth", inputType: "html5-date", required: true},
    play: async ({canvasElement}) => {
        const field = wrapper(canvasElement)
        expect(field).toHaveAttribute("data-input-type", "html5-date")
        expect(field).toHaveAttribute("data-required", "true")
        expect(within(canvasElement).getByRole("textbox", {name: /City/})).toBeRequired()
    },
}

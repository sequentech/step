// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {TimeZonePicker} from "./TimeZonePicker"

const meta = {
    title: "Admin/Timezones/TimeZonePicker",
    component: TimeZonePicker,
    args: {
        value: null,
        onChange: fn(),
        label: "Timezone",
        primary: "Asia/Manila",
        zones: ["Asia/Manila", "Europe/Madrid"],
        at: new Date("2028-01-01T00:00:00Z"),
    },
    decorators: [
        (Story: React.ComponentType) => (
            <AdminStoryProvider boundary={graphqlBoundary({})}>
                <Story />
            </AdminStoryProvider>
        ),
    ],
}
export default meta
type Story = StoryObj<typeof meta>
export const SearchAndChoose: Story = {
    parameters: {widgets: ["TimeZoneOptionRow"]},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.type(canvas.getByRole("combobox"), "Manila")
        const option = await within(document.body).findByRole("option")
        await expect(option).toHaveTextContent(/Manila/)
        await expect(option).toHaveTextContent(/primary/)
        await userEvent.click(option)
        await expect(args.onChange).toHaveBeenCalledWith("Asia/Manila")
    },
}
export const Disabled: Story = {
    args: {disabled: true},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("combobox")).toBeDisabled()
    },
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {ForensicCountry} from "./ForensicFields"

const meta = {
    title: "Admin/Election event/ForensicCountry",
    component: ForensicCountry,
    args: {code: "ES", showName: true},
} satisfies Meta<typeof ForensicCountry>
export default meta
type Story = StoryObj<typeof meta>

export const KnownCountry: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("ES")).toBeVisible()
        expect(canvas.getByText("Spain")).toBeVisible()
    },
}

export const UnknownCountry: Story = {
    args: {code: null},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("—")).toBeVisible()
        expect(within(canvasElement).queryByText("Spain")).toBeNull()
    },
}

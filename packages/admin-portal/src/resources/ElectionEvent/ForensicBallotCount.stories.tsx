// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {ForensicBallotCount} from "./ForensicFields"

const meta = {
    title: "Admin/Election event/ForensicBallotCount",
    component: ForensicBallotCount,
    args: {count: 3, shared: true},
} satisfies Meta<typeof ForensicBallotCount>
export default meta
type Story = StoryObj<typeof meta>

export const SharedAddress: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("3")).toBeVisible()
        expect(canvas.getByText("Shared")).toBeVisible()
    },
}

export const SingleBallot: Story = {
    args: {count: 1, shared: false},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("1")).toBeVisible()
        expect(canvas.queryByText("Shared")).toBeNull()
    },
}

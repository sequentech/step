// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {Messages} from "./Messages"

const meta = {
    title: "Admin/Screens/Messages",
    component: Messages,
} satisfies Meta<typeof Messages>
export default meta
type Story = StoryObj<typeof meta>

export const Placeholder: Story = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("Messages")).toBeVisible()
    },
}

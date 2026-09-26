// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {NoItem} from "./NoItem"

const meta = {title: "Admin/Components/NoItem", component: NoItem} satisfies Meta<typeof NoItem>
export default meta
type Story = StoryObj<typeof meta>

export const EmptyElectionList: Story = {
    args: {item: "No elections match these filters"},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText("No elections match these filters")
        ).toBeVisible()
    },
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {CustomTabPanel} from "./CustomTabPanel"

const meta = {
    title: "Admin/Components/CustomTabPanel",
    component: CustomTabPanel,
    args: {index: 1, value: 1, children: "Spanish name and alias fields"},
    argTypes: {children: {control: "text"}},
} satisfies Meta<typeof CustomTabPanel>
export default meta
type Story = StoryObj<typeof meta>

export const Selected: Story = {
    play: async ({canvasElement}) => {
        const panel = within(canvasElement).getByRole("tabpanel")
        await expect(panel).toHaveTextContent("Spanish name and alias fields")
        expect(panel).toHaveAttribute("id", "panel-tabpanel-1")
        expect(panel).toHaveAttribute("aria-labelledby", "panel-tab-1")
    },
}

export const OtherTabSelected: Story = {
    args: {value: 0},
    play: async ({canvasElement}) => {
        // The panel of an unselected tab is not rendered at all.
        expect(within(canvasElement).queryByRole("tabpanel", {hidden: true})).toBeNull()
        expect(within(canvasElement).queryByText("Spanish name and alias fields")).toBeNull()
    },
}

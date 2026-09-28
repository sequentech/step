// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {Stats} from "./Stats"

const meta = {
    title: "Admin/Dashboard/Election event/Stats",
    component: Stats,
    args: {
        metrics: {
            eligibleVotersCount: 12045,
            votersCount: 38,
            electionsCount: 2,
            areasCount: 4,
            emailsSentCount: 12,
            smsSentCount: 3,
        },
    },
} satisfies Meta<typeof Stats>
export default meta
type Story = StoryObj<typeof meta>

const card = (canvasElement: HTMLElement, label: string) =>
    within(canvasElement).getByText(label).parentElement as HTMLElement

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // Counts use the en-US grouping; the event dashboard shows no actual voters card.
        await expect(
            within(card(canvasElement, "Eligible Voters")).getByText("12,045")
        ).toBeVisible()
        await expect(within(card(canvasElement, "Elections")).getByText("2")).toBeVisible()
        await expect(within(card(canvasElement, "Areas")).getByText("4")).toBeVisible()
        await expect(within(card(canvasElement, "Emails sent")).getByText("12")).toBeVisible()
        await expect(within(card(canvasElement, "SMS sent")).getByText("3")).toBeVisible()
        expect(canvas.queryByText("Actual Voters")).not.toBeInTheDocument()
    },
}

export const Unknown: Story = {
    args: {
        metrics: {
            eligibleVotersCount: "-",
            votersCount: "-",
            electionsCount: "-",
            areasCount: "-",
            emailsSentCount: "-",
            smsSentCount: "-",
        },
    },
    play: async ({canvasElement}) => {
        expect(within(canvasElement).getAllByText("-")).toHaveLength(5)
    },
}

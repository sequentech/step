// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {Stats} from "./Stats"

const meta = {
    title: "Admin/Dashboard/Election/Stats",
    component: Stats,
    args: {
        metrics: {
            eligibleVotersCount: 1204,
            votersCount: 38,
            areasCount: 2,
            emailsSentCount: 12,
            smsSentCount: 0,
            messagesSentCount: {WHATSAPP: 40, VIBER: 7, MESSENGER: 0},
        },
    },
} satisfies Meta<typeof Stats>
export default meta
type Story = StoryObj<typeof meta>

const card = (canvasElement: HTMLElement, label: string) =>
    within(canvasElement).getByText(label).parentElement as HTMLElement

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(
            within(card(canvasElement, "Eligible Voters")).getByText("1,204")
        ).toBeVisible()
        await expect(within(card(canvasElement, "Actual Voters")).getByText("38")).toBeVisible()
        await expect(within(card(canvasElement, "Areas")).getByText("2")).toBeVisible()
        await expect(within(card(canvasElement, "Emails sent")).getByText("12")).toBeVisible()
        await expect(within(card(canvasElement, "SMS sent")).getByText("0")).toBeVisible()
        await expect(
            within(card(canvasElement, "WhatsApp messages sent")).getByText("40")
        ).toBeVisible()
        await expect(
            within(card(canvasElement, "Viber messages sent")).getByText("7")
        ).toBeVisible()
        await expect(
            within(card(canvasElement, "Messenger messages sent")).getByText("0")
        ).toBeVisible()
        expect(within(canvasElement).queryByText("Elections")).not.toBeInTheDocument()
    },
}

export const Unknown: Story = {
    args: {
        metrics: {
            eligibleVotersCount: "-",
            votersCount: "-",
            areasCount: "-",
            emailsSentCount: "-",
            smsSentCount: "-",
            messagesSentCount: {WHATSAPP: "-", VIBER: "-", MESSENGER: "-"},
        },
    },
    play: async ({canvasElement}) => {
        expect(within(canvasElement).getAllByText("-")).toHaveLength(8)
    },
}

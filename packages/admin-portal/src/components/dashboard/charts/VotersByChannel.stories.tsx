// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {VotingStatusChannel} from "@sequentech/ui-core"
import {VotersByChannel} from "./VotersByChannel"

const meta = {
    title: "Admin/Dashboard/Charts/VotersByChannel",
    component: VotersByChannel,
    args: {
        data: [
            {channel: VotingStatusChannel.Online, count: 30},
            {channel: VotingStatusChannel.Kiosk, count: 12},
            {channel: VotingStatusChannel.Telephone, count: 0},
        ],
        width: 360,
        height: 260,
    },
} satisfies Meta<typeof VotersByChannel>
export default meta
type Story = StoryObj<typeof meta>

const legend = (canvasElement: HTMLElement) =>
    Array.from(canvasElement.querySelectorAll(".apexcharts-legend-text")).map(
        (item) => item.textContent
    )

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Voters by channel")).toBeVisible()
        // A channel without voters is left out of the chart.
        await waitFor(() => expect(legend(canvasElement)).toEqual(["Online", "Kiosk"]))
        await waitFor(() =>
            expect(canvasElement.querySelector(".apexcharts-datalabels-group")).toHaveTextContent(
                "42"
            )
        )
    },
}

export const NoVoters: Story = {
    args: {data: []},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("Voters by channel")).toBeVisible()
        await waitFor(() =>
            expect(canvasElement.querySelector(".apexcharts-canvas")).not.toBeNull()
        )
        expect(legend(canvasElement)).toEqual([])
    },
}

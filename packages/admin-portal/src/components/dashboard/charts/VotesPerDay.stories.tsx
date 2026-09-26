// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {VotingStatusChannel} from "@sequentech/ui-core"
import type {CastVotesPerDay} from "@/gql/graphql"
import {VotesPerDay} from "./VotesPerDay"
import {DEFAULT_VOTES_TIME_SELECTION, VotesTimeRange, VotesTimeResolution} from "./votesTimeRange"

const vote = (day: string, channel: VotingStatusChannel, day_count: number): CastVotesPerDay => ({
    day,
    bucket: day,
    channel,
    day_count,
})

const meta = {
    title: "Admin/Dashboard/Charts/VotesPerDay",
    component: VotesPerDay,
    args: {
        data: [
            vote("2026-01-12", VotingStatusChannel.Online, 14),
            vote("2026-01-13", VotingStatusChannel.Online, 21),
            vote("2026-01-13", VotingStatusChannel.Kiosk, 5),
        ],
        width: 520,
        height: 260,
        selection: DEFAULT_VOTES_TIME_SELECTION,
        onSelectionChange: fn(),
    },
} satisfies Meta<typeof VotesPerDay>
export default meta
type Story = StoryObj<typeof meta>

const seriesNames = (canvasElement: HTMLElement) =>
    Array.from(canvasElement.querySelectorAll(".apexcharts-legend-text")).map(
        (item) => item.textContent
    )

export const Populated: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Votes over time")).toBeVisible()
        await expect(canvas.getByRole("combobox", {name: "Time resolution"})).toHaveTextContent(
            "Day"
        )
        await expect(canvas.getByRole("combobox", {name: "Time range"})).toHaveTextContent("7d")
        await waitFor(() => expect(seriesNames(canvasElement)).toEqual(["Online", "Kiosk"]))
        expect(canvas.queryByRole("progressbar")).not.toBeInTheDocument()
        expect(args.onSelectionChange).not.toHaveBeenCalled()
    },
}

export const Loading: Story = {
    args: {data: null},
    parameters: {
        expectedFailure: {
            reason: "The loading spinner is a progressbar without an accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("progressbar")).toBeVisible()
        // The controls stay usable while the votes load.
        await expect(canvas.getByRole("combobox", {name: "Time range"})).toBeVisible()
        expect(canvasElement.querySelector(".apexcharts-canvas")).toBeNull()
    },
}

export const Unavailable: Story = {
    args: {data: null, unavailable: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("-")).toBeVisible()
        expect(canvas.queryByRole("progressbar")).toBeNull()
        expect(canvasElement.querySelector(".apexcharts-canvas")).toBeNull()
    },
}

export const NoVotes: Story = {
    args: {data: []},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(canvasElement.querySelector(".apexcharts-canvas")).not.toBeNull()
        )
        expect(seriesNames(canvasElement)).toEqual([])
    },
}

export const ChangeResolution: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(
            within(canvasElement).getByRole("combobox", {name: "Time resolution"})
        )
        await userEvent.click(await within(document.body).findByRole("option", {name: "Hour"}))
        // A new resolution starts from its own default range.
        expect(args.onSelectionChange).toHaveBeenCalledWith({
            resolution: VotesTimeResolution.HOUR,
            range: VotesTimeRange.TWENTY_FOUR_HOURS,
        })
    },
}

export const ChangeRange: Story = {
    args: {selection: {resolution: VotesTimeResolution.MINUTE, range: VotesTimeRange.ONE_HOUR}},
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("combobox", {name: "Time range"}))
        const options = await within(document.body).findAllByRole("option")
        expect(options.map((option) => option.textContent)).toEqual(["15m", "1h", "6h"])
        await userEvent.click(within(document.body).getByRole("option", {name: "6h"}))
        expect(args.onSelectionChange).toHaveBeenCalledWith({
            resolution: VotesTimeResolution.MINUTE,
            range: VotesTimeRange.SIX_HOURS,
        })
    },
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {CreateContestData} from "./CreateContestData"
import {
    ContestLayout,
    dataWrites,
    reads,
    setUpContests,
    type ContestServices,
} from "./__stories__/ContestFixture"

const meta = {
    title: "Admin/Contest/CreateContestData",
    component: CreateContestData,
    args: {reads: "records", empty: false},
    parameters: {
        router: {
            path: "/sequent_backend_contest/create",
            initialEntries: ["/sequent_backend_contest/create"],
            layout: ContestLayout,
        },
    },
    beforeEach: ({args}) => setUpContests(args),
    render: () => <CreateContestData />,
} satisfies WidgetMeta<ContestServices>
export default meta
type Story = StoryObj<ContestServices>

export const Populated: Story = {
    parameters: {
        expectedFailure: {
            reason: "The progress indicator shown until the event arrives has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // The create screen starts without a contest, so the form never learns which
        // event to load and keeps waiting.
        await expect(canvas.getByRole("progressbar")).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
        expect(reads("getOne", "sequent_backend_election_event")).toEqual([])
        expect(dataWrites()).toEqual([])
    },
}

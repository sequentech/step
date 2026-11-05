// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {ContestBaseTabs} from "./ContestBaseTabs"
import {
    ContestLayout,
    reads,
    setUpContests,
    type ContestServices,
} from "./__stories__/ContestFixture"

const meta = {
    title: "Admin/Contest/ContestBaseTabs",
    component: ContestBaseTabs,
    args: {reads: "records", empty: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        router: {
            path: "/sequent_backend_contest/:id",
            initialEntries: [`/sequent_backend_contest/${STORY_IDS.contest}`],
            layout: ContestLayout,
        },
    },
    beforeEach: ({args}) => setUpContests(args),
    render: () => <ContestBaseTabs />,
} satisfies WidgetMeta<ContestServices>
export default meta
type Story = StoryObj<ContestServices>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Members")).toBeVisible()
        await expect(await canvas.findByDisplayValue("Council members")).toBeVisible()
        expect(reads("getOne", "sequent_backend_contest")[0].args[1]).toMatchObject({
            id: STORY_IDS.contest,
        })
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {
        expectedFailure: {
            reason: "The progress indicator shown until the record arrives has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getOne", "sequent_backend_contest")).toHaveLength(1))
        await expect(within(canvasElement).getByRole("progressbar")).toBeVisible()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Element does not exist")
        await waitFor(() => expect(message).toBeVisible())
        await expect(
            within(canvasElement).getByRole("status", {name: "Current location"})
        ).toHaveTextContent(/^\/sequent_backend_contest$/)
    },
}

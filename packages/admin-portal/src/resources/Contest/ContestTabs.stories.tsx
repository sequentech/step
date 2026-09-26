// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {ContestTabs} from "./ContestTabs"
import {
    ContestLayout,
    contests,
    reads,
    setUpContests,
    type ContestServices,
} from "./__stories__/ContestFixture"

interface Scenario extends ContestServices {
    /** Whether the show controller has the contest yet. */
    withRecord: boolean
}

const meta = {
    title: "Admin/Contest/ContestTabs",
    component: ContestTabs,
    args: {reads: "records", empty: false, withRecord: true},
    parameters: {
        router: {
            path: "/sequent_backend_contest/:id",
            initialEntries: [`/sequent_backend_contest/${STORY_IDS.contest}`],
            layout: ContestLayout,
        },
    },
    beforeEach: ({args}) => setUpContests(args),
    render: ({withRecord}) => (
        <RecordContextProvider value={withRecord ? contests()[0] : undefined}>
            <ContestTabs />
        </RecordContextProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("Members")).toBeVisible()
        await expect(canvas.getByText("Contest configuration.")).toBeVisible()
        await expect(canvas.getByRole("tab", {name: "Data"})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await expect(await canvas.findByDisplayValue("Council members")).toBeVisible()
    },
}

export const WithoutRecord: Story = {
    args: {withRecord: false},
    parameters: {
        expectedFailure: {
            reason: "The progress indicator shown until the record arrives has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("progressbar")).toBeVisible()
        expect(reads("getOne", "sequent_backend_contest")).toEqual([])
    },
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EditAreaContest} from "./EditAreaContest"
import {
    AreaContestFixture,
    dataWrites,
    reads,
    setUpAreaContests,
    type AreaContestServices,
} from "./__stories__/AreaContestFixture"

const meta = {
    title: "Admin/Area contest/EditAreaContest",
    component: EditAreaContest,
    args: {reads: "records", empty: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        router: {
            path: "/sequent_backend_area_contest/:id",
            initialEntries: [`/sequent_backend_area_contest/${STORY_IDS.areaContest}`],
        },
    },
    beforeEach: ({args}) => setUpAreaContests(args),
    render: () => (
        <AreaContestFixture>
            <EditAreaContest />
        </AreaContestFixture>
    ),
} satisfies WidgetMeta<AreaContestServices>
export default meta
type Story = StoryObj<AreaContestServices>

export const Populated: Story = {
    parameters: {
        widgets: ["AreaContestForm"],
        expectedFailure: {
            reason:
                "React-admin row selection labels a MUI 7 span instead of its checkbox, " +
                "and the JSON inputs grey their item counts below the contrast minimum.",
            a11y: ["aria-prohibited-attr", "color-contrast", "label"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // The form of the route's area contest sits beside the list of all of them.
        await expect(await canvas.findByText(STORY_IDS.areaContest)).toBeVisible()
        await expect(await canvas.findByRole("row", {name: /South district/})).toBeVisible()
        expect(reads("getOne", "sequent_backend_area_contest")[0].args[1]).toMatchObject({
            id: STORY_IDS.areaContest,
        })
        // Nothing has changed yet, so the form cannot be saved.
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(dataWrites()).toEqual([])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getOne", "sequent_backend_area_contest")).toHaveLength(1))
        expect(within(canvasElement).queryByText(STORY_IDS.areaContest)).toBeNull()
    },
}

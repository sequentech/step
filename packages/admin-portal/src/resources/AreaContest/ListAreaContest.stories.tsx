// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {ListAreaContest} from "./ListAreaContest"
import {
    AreaContestFixture,
    reads,
    setUpAreaContests,
    southAreaContest,
    type AreaContestServices,
} from "./__stories__/AreaContestFixture"

const meta = {
    title: "Admin/Area contest/ListAreaContest",
    component: ListAreaContest,
    args: {reads: "records", empty: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "React-admin row selection labels a MUI 7 span instead of its checkbox.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: ({args}) => setUpAreaContests(args),
    render: () => (
        <AreaContestFixture>
            <ListAreaContest />
        </AreaContestFixture>
    ),
} satisfies WidgetMeta<AreaContestServices>
export default meta
type Story = StoryObj<AreaContestServices>

const row = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(name)})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const north = await row(canvasElement, "North district")
        await expect(await row(canvasElement, "South district")).toBeVisible()
        // Event and contest names live in the presentation since migration 1772358027729.
        await expect(await within(north).findByText("Council event")).toBeVisible()
        await expect(await within(north).findByText("Council members")).toBeVisible()
        expect(reads("getList", "sequent_backend_area_contest")[0].args[1]).toMatchObject({
            filter: {tenant_id: TENANT_ID},
        })
        const [{args: areaRead}] = reads("getMany", "sequent_backend_area")
        expect([...(areaRead[1] as {ids: string[]}).ids].sort()).toEqual([
            STORY_IDS.area,
            STORY_IDS.secondArea,
        ])
    },
}

export const Empty: Story = {
    args: {empty: true},
    parameters: {
        expectedFailure: {
            reason: "React-admin's default empty page greys its message below the contrast minimum.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("No Area Contest yet.")).toBeVisible()
        expect(within(canvasElement).queryByRole("row", {name: /district/})).toBeNull()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(reads("getList", "sequent_backend_area_contest")).toHaveLength(1)
        )
        expect(within(canvasElement).queryByRole("row", {name: /district/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /district/})).toBeNull()
    },
}

export const RowOpensTheAreaContest: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(await row(canvasElement, "South district"))
        const filter = JSON.stringify({
            election_event_id: southAreaContest.election_event_id,
            contest_id: southAreaContest.contest_id,
            area_id: southAreaContest.area_id,
        })
        await waitFor(() =>
            expect(
                within(canvasElement).getByRole("status", {name: "Current location"})
            ).toHaveTextContent(
                `/sequent_backend_area_contest/${southAreaContest.id}?filter=${filter}`
            )
        )
    },
}

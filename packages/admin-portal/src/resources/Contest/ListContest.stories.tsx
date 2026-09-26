// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {ListContest} from "./ListContest"
import {
    ContestScreen,
    reads,
    setUpContests,
    type ContestServices,
} from "./__stories__/ContestFixture"

const listDefects = {
    expectedFailure: {
        reason: "React-admin row selection labels a MUI 7 span instead of its checkbox.",
        a11y: ["aria-prohibited-attr", "label"],
    },
}

const meta = {
    title: "Admin/Contest/ListContest",
    component: ListContest,
    args: {reads: "records", empty: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: listDefects,
    beforeEach: ({args}) => setUpContests(args),
    render: () => (
        <ContestScreen>
            <ListContest />
        </ContestScreen>
    ),
} satisfies WidgetMeta<ContestServices>
export default meta
type Story = StoryObj<ContestServices>

const row = (canvasElement: HTMLElement, description: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(description)})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const council = await row(canvasElement, "Select up to two members")
        await expect(await row(canvasElement, "Choose the chair")).toBeVisible()
        // Names live in the presentation since migration 1772358027729 dropped the name column.
        await expect(within(council).getByText("Council members")).toBeVisible()
        await expect(await within(council).findByText("Council event")).toBeVisible()
        await expect(await within(council).findByText("Council election")).toBeVisible()
        // Each contest's candidates are chips linking to the candidate.
        await waitFor(() => expect(within(council).getAllByRole("link")).toHaveLength(2))
        const targets = within(council)
            .getAllByRole("link")
            .map((link) => link.getAttribute("href")?.split("?")[0])
        expect(targets.sort()).toEqual([
            `/sequent_backend_candidate/${STORY_IDS.candidate}`,
            `/sequent_backend_candidate/${STORY_IDS.secondCandidate}`,
        ])
        expect(reads("getList", "sequent_backend_contest")[0].args[1]).toMatchObject({
            filter: {tenant_id: TENANT_ID},
        })
        expect(reads("getManyReference", "sequent_backend_candidate").length).toBeGreaterThan(0)
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
        await expect(await within(canvasElement).findByText("No Contests yet.")).toBeVisible()
        expect(within(canvasElement).queryByRole("row", {name: /members/})).toBeNull()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getList", "sequent_backend_contest")).toHaveLength(1))
        expect(within(canvasElement).queryByRole("row", {name: /members/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /members/})).toBeNull()
    },
}

export const SearchByName: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await row(canvasElement, "Select up to two members")
        await userEvent.click(canvas.getByRole("button", {name: "Add filter"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitemcheckbox", {name: "Name"})
        )
        await userEvent.type(await canvas.findByRole("textbox", {name: "Name"}), "chair")
        await waitFor(() =>
            expect(canvas.queryByRole("row", {name: /Select up to two members/})).toBeNull()
        )
        await expect(await row(canvasElement, "Choose the chair")).toBeVisible()
        expect(reads("getList", "sequent_backend_contest").at(-1)?.args[1]).toMatchObject({
            filter: {tenant_id: TENANT_ID, _or: {format: "hasura-raw-query"}},
        })
        await expect(canvas.getByRole("textbox", {name: "Name"})).toHaveValue("chair")
    },
}

export const RowOpensTheContest: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(await row(canvasElement, "Choose the chair"))
        const filter = JSON.stringify({
            election_event_id: STORY_IDS.event,
            election_id: STORY_IDS.election,
        })
        await waitFor(() =>
            expect(
                within(canvasElement).getByRole("status", {name: "Current location"})
            ).toHaveTextContent(
                `/sequent_backend_contest/${STORY_IDS.secondContest}?filter=${filter}`
            )
        )
    },
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {ListElection} from "./ListElection"
import {
    ElectionLayout,
    reads,
    setUpElections,
    type ElectionServices,
} from "./__stories__/ElectionFixture"

const meta = {
    title: "Admin/Election/ListElection",
    component: ListElection,
    args: {reads: "records", empty: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        router: {
            path: "/sequent_backend_election",
            initialEntries: ["/sequent_backend_election"],
            layout: ElectionLayout,
        },
        expectedFailure: {
            reason: "React-admin row selection labels a MUI 7 span instead of its checkbox.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: ({args}) => setUpElections(args),
    render: () => <ListElection />,
} satisfies WidgetMeta<ElectionServices>
export default meta
type Story = StoryObj<ElectionServices>

const row = (canvasElement: HTMLElement, description: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(description)})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const council = await row(canvasElement, "Choose the council members")
        await expect(council).toBeVisible()
        await expect(await row(canvasElement, "Choose the mayor")).toBeVisible()
        // Names live in the presentation since elections and events lost their name column.
        await expect(within(council).getByText("Council election")).toBeVisible()
        await expect(await within(council).findByText("Council event")).toBeVisible()
        // The contest chips link to the council election's contests.
        await waitFor(() =>
            expect(
                within(council)
                    .getAllByRole("link")
                    .map((link) => new URL(link.getAttribute("href") ?? "", "http://x").pathname)
                    .sort()
            ).toEqual(
                [
                    `/sequent_backend_contest/${STORY_IDS.contest}`,
                    `/sequent_backend_contest/${STORY_IDS.secondContest}`,
                ].sort()
            )
        )
        expect(reads("getList", "sequent_backend_election")[0].args[1]).toMatchObject({
            filter: {tenant_id: TENANT_ID},
        })
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
        await expect(await within(canvasElement).findByText("No Elections yet.")).toBeVisible()
        expect(within(canvasElement).queryByRole("row", {name: /Choose/})).toBeNull()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getList", "sequent_backend_election")).toHaveLength(1))
        await expect(within(canvasElement).getByText("Elections")).toBeVisible()
        expect(within(canvasElement).queryByRole("row", {name: /Choose/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /Choose/})).toBeNull()
    },
}

export const SearchByName: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await row(canvasElement, "Choose the council members")
        await userEvent.click(canvas.getByRole("button", {name: "Add filter"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitemcheckbox", {name: "Name"})
        )
        await userEvent.type(await canvas.findByRole("textbox", {name: "Name"}), "Mayoral")
        await waitFor(() =>
            expect(canvas.queryByRole("row", {name: /Choose the council members/})).toBeNull()
        )
        await expect(await row(canvasElement, "Choose the mayor")).toBeVisible()
        expect(reads("getList", "sequent_backend_election").at(-1)?.args[1]).toMatchObject({
            filter: {tenant_id: TENANT_ID, _or: {format: "hasura-raw-query"}},
        })
        await expect(canvas.getByRole("textbox", {name: "Name"})).toHaveValue("Mayoral")
    },
}

export const RowOpensTheElection: Story = {
    // The election's own route leaves the list, where axe finds no defect.
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await userEvent.click(await row(canvasElement, "Choose the mayor"))
        const filter = JSON.stringify({election_event_id: STORY_IDS.event})
        await waitFor(() =>
            expect(
                within(canvasElement).getByRole("status", {name: "Current location"})
            ).toHaveTextContent(
                `/sequent_backend_election/${STORY_IDS.secondElection}?filter=${filter}`
            )
        )
    },
}

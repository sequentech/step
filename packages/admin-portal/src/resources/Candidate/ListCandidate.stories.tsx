// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {ListCandidate} from "./ListCandidate"
import {
    CandidateScreen,
    reads,
    setUpCandidates,
    type CandidateServices,
} from "./__stories__/CandidateFixture"

const meta = {
    title: "Admin/Candidate/ListCandidate",
    component: ListCandidate,
    args: {reads: "records", empty: false, withImage: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "React-admin row selection labels a MUI 7 span instead of its checkbox.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: ({args}) => setUpCandidates(args),
    render: () => (
        <CandidateScreen>
            <ListCandidate />
        </CandidateScreen>
    ),
} satisfies WidgetMeta<CandidateServices>
export default meta
type Story = StoryObj<CandidateServices>

const row = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(name)})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const alice = await row(canvasElement, "Alice Example")
        await expect(within(alice).getByText("Alice Example stands for the council")).toBeVisible()
        // Names live in the presentation since migration 1772358027729 dropped the name column.
        await expect(within(alice).getByText("Alice Example")).toBeVisible()
        await expect(await within(alice).findByText("Council event")).toBeVisible()
        await expect(await within(alice).findByText("Council members")).toBeVisible()
        expect(reads("getMany", "sequent_backend_contest")).toHaveLength(1)
        expect(reads("getMany", "sequent_backend_election_event")).toHaveLength(1)
        await expect(await row(canvasElement, "Bob Example")).toBeVisible()
        expect(reads("getList", "sequent_backend_candidate")[0].args[1]).toMatchObject({
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
        await expect(await within(canvasElement).findByText("No Candidates yet.")).toBeVisible()
        expect(within(canvasElement).queryByRole("row", {name: /Example/})).toBeNull()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getList", "sequent_backend_candidate")).toHaveLength(1))
        expect(within(canvasElement).queryByRole("row", {name: /Example/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /Example/})).toBeNull()
    },
}

export const SearchByName: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await row(canvasElement, "Alice Example")
        await userEvent.click(canvas.getByRole("button", {name: "Add filter"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitemcheckbox", {name: "Name"})
        )
        await userEvent.type(await canvas.findByRole("textbox", {name: "Name"}), "Bob")
        await waitFor(() => expect(canvas.queryByRole("row", {name: /Alice Example/})).toBeNull())
        await expect(await row(canvasElement, "Bob Example")).toBeVisible()
        expect(reads("getList", "sequent_backend_candidate").at(-1)?.args[1]).toMatchObject({
            filter: {tenant_id: TENANT_ID, _or: {format: "hasura-raw-query"}},
        })
        await expect(canvas.getByRole("textbox", {name: "Name"})).toHaveValue("Bob")
    },
}

export const RowOpensTheCandidate: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(await row(canvasElement, "Bob Example"))
        const filter = JSON.stringify({
            election_event_id: STORY_IDS.event,
            contest_id: STORY_IDS.contest,
        })
        await waitFor(() =>
            expect(
                within(canvasElement).getByRole("status", {name: "Current location"})
            ).toHaveTextContent(
                `/sequent_backend_candidate/${STORY_IDS.secondCandidate}?filter=${filter}`
            )
        )
    },
}

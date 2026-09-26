// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EditElection} from "./EditElection"
import {
    ElectionLayout,
    dataWrites,
    graphqlCalls,
    reads,
    setUpElections,
    type ElectionServices,
} from "./__stories__/ElectionFixture"

const meta = {
    title: "Admin/Election/EditElection",
    component: EditElection,
    args: {reads: "records", empty: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        router: {
            path: "/sequent_backend_election/:id",
            initialEntries: [`/sequent_backend_election/${STORY_IDS.election}`],
            layout: ElectionLayout,
        },
        expectedFailure: {
            reason:
                "React-admin row selection labels a MUI 7 span instead of its checkbox, " +
                "the JSON inputs grey their item counts below the contrast minimum, and " +
                "the Add contest link wraps a button.",
            a11y: ["aria-prohibited-attr", "color-contrast", "label", "nested-interactive"],
        },
    },
    beforeEach: ({args}) =>
        setUpElections(args, {
            CreateScheduledEvent: () => ({data: {createScheduledEvent: {id: "scheduled-1"}}}),
        }),
    render: () => <EditElection />,
} satisfies WidgetMeta<ElectionServices>
export default meta
type Story = StoryObj<ElectionServices>

const descriptionInput = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("textbox", {name: "Description"})

export const Populated: Story = {
    parameters: {widgets: ["ElectionForm"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Election configuration")).toBeVisible()
        await waitFor(async () =>
            expect(await descriptionInput(canvasElement)).toHaveValue("Choose the council members")
        )
        // Names live in the presentation since migration 1772358027729.
        const form = within(canvas.getByRole("textbox", {name: "Name"}).closest("form")!)
        expect(form.getByRole("textbox", {name: "Name"})).toHaveValue("Council election")
        await expect(await form.findByText("Council event")).toBeVisible()
        expect(canvas.getAllByRole("row", {name: /Choose/})).toHaveLength(2)
        expect(reads("getOne", "sequent_backend_election")[0].args[1]).toMatchObject({
            id: STORY_IDS.election,
        })
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getOne", "sequent_backend_election")).toHaveLength(1))
        await expect(within(canvasElement).getByText("Elections")).toBeVisible()
        expect(within(canvasElement).queryByText("Election configuration")).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Element does not exist")
        await waitFor(() => expect(message).toBeVisible())
        await expect(
            within(canvasElement).getByRole("status", {name: "Current location"})
        ).toHaveTextContent(/^\/sequent_backend_election$/)
    },
}

export const OpenTheVoting: Story = {
    parameters: {widgets: ["ElectionForm"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("Election configuration")
        await userEvent.click(canvas.getByRole("button", {name: "Actions"}))
        const menu = within(await within(document.body).findByRole("menu"))
        // The council election has closed, so it can only be opened again.
        await expect(menu.getByRole("menuitem", {name: "Pause Voting"})).toHaveAttribute(
            "aria-disabled",
            "true"
        )
        await expect(menu.getByRole("menuitem", {name: "Close Voting"})).toHaveAttribute(
            "aria-disabled",
            "true"
        )
        await userEvent.click(menu.getByRole("menuitem", {name: "Open Voting"}))
        await waitFor(() =>
            expect(graphqlCalls().filter(({name}) => name === "CreateScheduledEvent")).toHaveLength(
                1
            )
        )
        expect(
            graphqlCalls().find(({name}) => name === "CreateScheduledEvent")?.variables
        ).toEqual({
            tenantId: TENANT_ID,
            electionEventId: STORY_IDS.event,
            eventProcessor: "UPDATE_VOTING_STATUS",
            eventPayload: {election_id: STORY_IDS.election, status: "OPEN"},
        })
        // The screen reloads the election after scheduling the change.
        await waitFor(() => expect(reads("getOne", "sequent_backend_election")).toHaveLength(2))
        expect(dataWrites()).toEqual([])
    },
}

export const RenameTheElection: Story = {
    parameters: {widgets: ["ElectionForm"], expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const name = await canvas.findByRole("textbox", {name: "Name"})
        await waitFor(() => expect(name).toHaveValue("Council election"))
        await userEvent.clear(name)
        await userEvent.type(name, "Council vote")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        const notification = await within(document.body).findByText("Element updated")
        await userEvent.click(canvasElement)
        await waitFor(() => expect(notification).not.toBeInTheDocument())
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        // The name is written to the presentation of the default language.
        expect(dataWrites()[0].params.data).not.toHaveProperty("name")
        expect(dataWrites()[0].params).toMatchObject({
            id: STORY_IDS.election,
            data: {presentation: {i18n: {en: {name: "Council vote", alias: "Council"}}}},
        })
    },
}

export const DescribeTheElection: Story = {
    // Saving returns to the list route, which the story does not render.
    parameters: {widgets: ["ElectionForm"], expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const description = await descriptionInput(canvasElement)
        await waitFor(() => expect(description).toHaveValue("Choose the council members"))
        await userEvent.clear(description)
        await userEvent.type(description, "Choose the new council")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        // The update is undoable: it reaches the service once its notification closes.
        const notification = await within(document.body).findByText("Element updated")
        expect(dataWrites()).toEqual([])
        await waitFor(() =>
            expect(canvas.getByRole("status", {name: "Current location"})).toHaveTextContent(
                /^\/sequent_backend_election$/
            )
        )
        await userEvent.click(canvasElement)
        await waitFor(() => expect(notification).not.toBeInTheDocument())
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        expect(dataWrites()[0]).toEqual({
            method: "update",
            resource: "sequent_backend_election",
            params: expect.objectContaining({
                id: STORY_IDS.election,
                data: expect.objectContaining({description: "Choose the new council"}),
            }),
        })
    },
}

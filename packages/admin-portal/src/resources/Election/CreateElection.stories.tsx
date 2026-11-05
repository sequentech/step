// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {EVENT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {LANGUAGE_CONF, storyId} from "@/__stories__/fixtures"
import {CreateElection} from "./CreateElection"
import {
    ElectionLayout,
    dataWrites,
    electionFlag,
    graphqlCalls,
    lastCreated,
    setUpElections,
    type ElectionServices,
} from "./__stories__/ElectionFixture"

interface Scenario extends ElectionServices {
    /** Whether the create_election action refuses the election. */
    rejected: boolean
}

const CREATED_ID = storyId(3, 5)

const meta = {
    title: "Admin/Election/CreateElection",
    component: CreateElection,
    args: {reads: "records", empty: false, rejected: false},
    parameters: {
        router: {
            path: "/sequent_backend_election/create",
            initialEntries: [`/sequent_backend_election/create?electionEventId=${EVENT_ID}`],
            layout: ElectionLayout,
        },
    },
    beforeEach: ({args}) =>
        setUpElections(args, {
            CreateElection: () =>
                args.rejected
                    ? {errors: [new GraphQLError("Synthetic election rejected")]}
                    : {data: {create_election: {id: CREATED_ID}}},
        }),
    render: () => <CreateElection />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const creations = () => graphqlCalls().filter(({name}) => name === "CreateElection")

async function fillIn(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await userEvent.type(await canvas.findByRole("textbox", {name: /^Name/}), "Mayoral election")
    await userEvent.type(canvas.getByRole("textbox", {name: /^External ID/}), "mayor-2026")
    await userEvent.type(canvas.getByRole("textbox", {name: "Description"}), "Choose the mayor")
    await userEvent.click(canvas.getByRole("button", {name: "Save"}))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Election")).toBeVisible()
        await expect(canvas.getByRole("textbox", {name: /^Name/})).toHaveValue("")
        // A pristine form cannot be saved.
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(creations()).toEqual([])
    },
}

export const CreateInTheEvent: Story = {
    play: async ({canvasElement}) => {
        await fillIn(canvasElement)
        await waitFor(() => expect(creations()).toHaveLength(1))
        // The election takes the event's languages; the name only reaches the translations.
        expect(creations()[0].variables).toEqual({
            electionEventId: EVENT_ID,
            externalId: "mayor-2026",
            description: "Choose the mayor",
            presentation: expect.objectContaining({
                i18n: expect.objectContaining({
                    en: {name: "Mayoral election", description: "Choose the mayor"},
                }),
                language_conf: LANGUAGE_CONF,
            }),
        })
        await waitFor(() =>
            expect(
                within(canvasElement).getByRole("status", {name: "Current location"})
            ).toHaveTextContent(`/sequent_backend_election/${CREATED_ID}`)
        )
        expect(lastCreated).toHaveBeenCalledWith({id: CREATED_ID, type: "sequent_backend_election"})
        expect(electionFlag).toHaveBeenCalledWith(CREATED_ID)
        // The menu tree reloads to show the new election.
        expect(
            graphqlCalls().filter(({name}) => name === "election_events_tree").length
        ).toBeGreaterThan(1)
        expect(dataWrites()).toEqual([])
    },
}

export const Rejected: Story = {
    args: {rejected: true},
    play: async ({canvasElement}) => {
        await fillIn(canvasElement)
        await waitFor(() => expect(creations()).toHaveLength(1))
        const message = await within(document.body).findByText("Synthetic election rejected")
        await waitFor(() => expect(message).toBeVisible())
        await expect(
            within(canvasElement).getByRole("status", {name: "Current location"})
        ).toHaveTextContent("/sequent_backend_election/create")
        expect(lastCreated).not.toHaveBeenCalled()
        expect(electionFlag).not.toHaveBeenCalled()
    },
}

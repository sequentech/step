// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {CreateContest} from "./CreateContest"
import {
    ContestLayout,
    contestFlag,
    dataWrites,
    graphqlCalls,
    lastCreated,
    setUpContests,
    type ContestServices,
} from "./__stories__/ContestFixture"

const meta = {
    title: "Admin/Contest/CreateContest",
    component: CreateContest,
    args: {reads: "records", empty: false},
    parameters: {
        router: {
            path: "/sequent_backend_contest/create",
            initialEntries: [
                `/sequent_backend_contest/create?electionEventId=${EVENT_ID}&electionId=${STORY_IDS.election}`,
            ],
            layout: ContestLayout,
        },
    },
    beforeEach: ({args}) => setUpContests(args),
    render: () => <CreateContest />,
} satisfies WidgetMeta<ContestServices>
export default meta
type Story = StoryObj<ContestServices>

async function fillIn(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await userEvent.type(await canvas.findByRole("textbox", {name: "Name"}), "Treasurer")
    await userEvent.type(canvas.getByRole("textbox", {name: "Description"}), "Choose one")
    await userEvent.click(canvas.getByRole("button", {name: "Save"}))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Create a Contest")).toBeVisible()
        await expect(canvas.getByRole("textbox", {name: "Name"})).toHaveValue("")
        // A pristine form cannot be saved.
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(dataWrites()).toEqual([])
    },
}

export const CreateInTheElection: Story = {
    play: async ({canvasElement}) => {
        await fillIn(canvasElement)
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        // The hidden inputs give a plurality contest with one seat; the form still
        // sends a name, which the contest table no longer has.
        expect(dataWrites()[0]).toEqual({
            method: "create",
            resource: "sequent_backend_contest",
            params: expect.objectContaining({
                data: expect.objectContaining({
                    name: "Treasurer",
                    description: "Choose one",
                    tenant_id: TENANT_ID,
                    election_event_id: EVENT_ID,
                    election_id: STORY_IDS.election,
                    min_votes: 0,
                    max_votes: 1,
                    winning_candidates_num: 1,
                    counting_algorithm: "plurality-at-large",
                    is_encrypted: true,
                    is_active: true,
                    presentation: expect.objectContaining({
                        allow_writeins: true,
                        candidates_order: "alphabetical",
                        i18n: expect.objectContaining({
                            en: {name: "Treasurer", description: "Choose one"},
                        }),
                    }),
                }),
            }),
        })
        await waitFor(() =>
            expect(
                within(canvasElement).getByRole("status", {name: "Current location"})
            ).toHaveTextContent("/sequent_backend_contest/created-1")
        )
        expect(lastCreated).toHaveBeenCalledWith({id: "created-1", type: "sequent_backend_contest"})
        expect(contestFlag).toHaveBeenCalledWith("created-1")
        // The menu tree reloads to show the new contest.
        expect(graphqlCalls().map(({name}) => name)).toEqual([
            "election_events_tree",
            "election_events_tree",
        ])
    },
}

export const SaveFailure: Story = {
    args: {writeError: "Synthetic contest rejected"},
    play: async ({canvasElement}) => {
        await fillIn(canvasElement)
        const message = await within(document.body).findByText("Synthetic contest rejected")
        await waitFor(() => expect(message).toBeVisible())
        expect(dataWrites().map(({method}) => method)).toEqual(["create"])
        expect(lastCreated).not.toHaveBeenCalled()
        await expect(
            within(canvasElement).getByRole("status", {name: "Current location"})
        ).toHaveTextContent("/sequent_backend_contest/create")
    },
}

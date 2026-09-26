// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {CreateCandidate} from "./CreateCandidate"
import {
    CandidateLayout,
    dataWrites,
    graphqlCalls,
    lastCreated,
    setUpCandidates,
    tallyFlags,
    type CandidateServices,
} from "./__stories__/CandidateFixture"

const meta = {
    title: "Admin/Candidate/CreateCandidate",
    component: CreateCandidate,
    args: {reads: "records", empty: false, withImage: false},
    parameters: {
        router: {
            path: "/sequent_backend_candidate/create",
            initialEntries: [
                `/sequent_backend_candidate/create?electionEventId=${EVENT_ID}&contestId=${STORY_IDS.contest}`,
            ],
            layout: CandidateLayout,
        },
    },
    beforeEach: ({args}) => setUpCandidates(args),
    render: () => <CreateCandidate />,
} satisfies WidgetMeta<CandidateServices>
export default meta
type Story = StoryObj<CandidateServices>

async function fillIn(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await userEvent.type(await canvas.findByRole("textbox", {name: "Name"}), "Carol Example")
    await userEvent.type(canvas.getByRole("textbox", {name: "Description"}), "Carol stands")
    await userEvent.click(canvas.getByRole("button", {name: "Save"}))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Create a Candidate")).toBeVisible()
        await expect(canvas.getByRole("textbox", {name: "Name"})).toHaveValue("")
        // A pristine form cannot be saved.
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(dataWrites()).toEqual([])
    },
}

export const CreateInTheContest: Story = {
    play: async ({canvasElement}) => {
        await fillIn(canvasElement)
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        // The form still sends a name, which the candidate table no longer has.
        expect(dataWrites()[0]).toEqual({
            method: "create",
            resource: "sequent_backend_candidate",
            params: expect.objectContaining({
                data: expect.objectContaining({
                    name: "Carol Example",
                    description: "Carol stands",
                    tenant_id: TENANT_ID,
                    election_event_id: EVENT_ID,
                    contest_id: STORY_IDS.contest,
                    presentation: expect.objectContaining({
                        i18n: expect.objectContaining({
                            en: {name: "Carol Example", description: "Carol stands"},
                        }),
                    }),
                }),
            }),
        })
        await waitFor(() =>
            expect(
                within(canvasElement).getByRole("status", {name: "Current location"})
            ).toHaveTextContent("/sequent_backend_candidate/created-1")
        )
        expect(lastCreated).toHaveBeenCalledWith({
            id: "created-1",
            type: "sequent_backend_candidate",
        })
        expect(tallyFlags.candidate).toHaveBeenCalledWith("created-1")
        // The menu tree reloads to show the new candidate.
        expect(graphqlCalls().map(({name}) => name)).toEqual([
            "election_events_tree",
            "election_events_tree",
        ])
    },
}

export const SaveFailure: Story = {
    args: {writeError: "Synthetic candidate rejected"},
    play: async ({canvasElement}) => {
        await fillIn(canvasElement)
        const message = await within(document.body).findByText("Synthetic candidate rejected")
        await waitFor(() => expect(message).toBeVisible())
        expect(dataWrites().map(({method}) => method)).toEqual(["create"])
        expect(lastCreated).not.toHaveBeenCalled()
        await expect(
            within(canvasElement).getByRole("status", {name: "Current location"})
        ).toHaveTextContent("/sequent_backend_candidate/create")
    },
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EditCandidate} from "./EditCandidate"
import {
    CandidateLayout,
    dataWrites,
    reads,
    setUpCandidates,
    type CandidateServices,
} from "./__stories__/CandidateFixture"

const formDefects = {
    expectedFailure: {
        reason:
            "React-admin row selection labels a MUI 7 span instead of its checkbox, " +
            "and the JSON inputs grey their item counts below the contrast minimum.",
        a11y: ["aria-prohibited-attr", "color-contrast", "label"],
    },
}

const meta = {
    title: "Admin/Candidate/EditCandidate",
    component: EditCandidate,
    args: {reads: "records", empty: false, withImage: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        router: {
            path: "/sequent_backend_candidate/:id",
            initialEntries: [`/sequent_backend_candidate/${STORY_IDS.candidate}`],
            layout: CandidateLayout,
        },
    },
    beforeEach: ({args}) => setUpCandidates(args),
    render: () => <EditCandidate />,
} satisfies WidgetMeta<CandidateServices>
export default meta
type Story = StoryObj<CandidateServices>

const descriptionInput = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("textbox", {name: "Description"})

export const Populated: Story = {
    parameters: {widgets: ["CandidateForm"], ...formDefects},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Candidate configuration")).toBeVisible()
        await waitFor(async () =>
            expect(await descriptionInput(canvasElement)).toHaveValue(
                "Alice Example stands for the council"
            )
        )
        // The candidate table has no name column, so the Name input starts empty.
        expect(canvas.getByRole("textbox", {name: "Name"})).toHaveValue("")
        await expect(await canvas.findByRole("combobox", {name: "Contest"})).toBeVisible()
        expect(reads("getOne", "sequent_backend_candidate")[0].args[1]).toMatchObject({
            id: STORY_IDS.candidate,
        })
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getOne", "sequent_backend_candidate")).toHaveLength(1))
        expect(within(canvasElement).queryByRole("textbox", {name: "Name"})).toBeNull()
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
        ).toHaveTextContent(/^\/sequent_backend_candidate$/)
    },
}

export const DescribeTheCandidate: Story = {
    // Saving returns to the empty list route, where axe finds no defect of the form.
    parameters: {widgets: ["CandidateForm"], expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const description = await descriptionInput(canvasElement)
        await waitFor(() => expect(description).toHaveValue("Alice Example stands for the council"))
        await userEvent.clear(description)
        await userEvent.type(description, "Alice stands again")
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        // The update is undoable: it reaches the service once its notification closes.
        const notification = await within(document.body).findByText("Element updated")
        expect(dataWrites()).toEqual([])
        await waitFor(() =>
            expect(canvas.getByRole("status", {name: "Current location"})).toHaveTextContent(
                /^\/sequent_backend_candidate$/
            )
        )
        await userEvent.click(canvasElement)
        await waitFor(() => expect(notification).not.toBeInTheDocument())
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        expect(dataWrites()[0]).toEqual({
            method: "update",
            resource: "sequent_backend_candidate",
            params: expect.objectContaining({
                id: STORY_IDS.candidate,
                data: expect.objectContaining({description: "Alice stands again"}),
            }),
        })
    },
}

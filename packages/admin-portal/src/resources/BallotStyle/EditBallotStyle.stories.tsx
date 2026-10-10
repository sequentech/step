// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {EditBallotStyle} from "./EditBallotStyle"
import {
    BallotStyleLayout,
    dataWrites,
    northBallot,
    reads,
    setUpBallotStyles,
    type BallotStyleServices,
} from "./__stories__/BallotStyleFixture"

const formDefects = {
    expectedFailure: {
        reason:
            "React-admin row selection labels a MUI 7 span instead of its checkbox, " +
            "and the JSON inputs grey their item counts below the contrast minimum.",
        a11y: ["aria-prohibited-attr", "color-contrast", "label"],
    },
}

const meta = {
    title: "Admin/Ballot style/EditBallotStyle",
    component: EditBallotStyle,
    args: {reads: "records", empty: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        router: {
            path: "/sequent_backend_ballot_style/:id",
            initialEntries: [`/sequent_backend_ballot_style/${northBallot.id}`],
            layout: BallotStyleLayout,
        },
    },
    beforeEach: ({args}) => setUpBallotStyles(args),
    render: () => <EditBallotStyle />,
} satisfies WidgetMeta<BallotStyleServices>
export default meta
type Story = StoryObj<BallotStyleServices>

const areaSelect = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("combobox", {name: "Area"})

export const Populated: Story = {
    parameters: {widgets: ["BallotStyleForm"], ...formDefects},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText(northBallot.id)).toBeVisible()
        // Election and event names live in the presentation since migration 1772358027729.
        const form = within(canvas.getByText(northBallot.id).closest("form")!)
        await expect(await form.findByText("Council election")).toBeVisible()
        await expect(await form.findByText("Council event")).toBeVisible()
        await expect(await areaSelect(canvasElement)).toHaveTextContent("North district")
        expect(reads("getOne", "sequent_backend_ballot_style")[0].args[1]).toMatchObject({
            id: northBallot.id,
        })
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getOne", "sequent_backend_ballot_style")).toHaveLength(1))
        expect(within(canvasElement).queryByText(northBallot.id)).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Element does not exist")
        await waitFor(() => expect(message).toBeVisible())
        expect(reads("getOne", "sequent_backend_ballot_style")).toHaveLength(1)
        await expect(
            within(canvasElement).getByRole("status", {name: "Current location"})
        ).toHaveTextContent(/^\/sequent_backend_ballot_style$/)
    },
}

export const MoveToAnotherArea: Story = {
    // Saving returns to the empty list route, where axe finds no defect of the form.
    parameters: {widgets: ["BallotStyleForm"], expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await areaSelect(canvasElement))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "South district"})
        )
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        // The update is undoable: it reaches the service once its notification closes.
        const notification = await within(document.body).findByText("Element updated")
        expect(dataWrites()).toEqual([])
        await waitFor(() =>
            expect(canvas.getByRole("status", {name: "Current location"})).toHaveTextContent(
                /^\/sequent_backend_ballot_style$/
            )
        )
        await userEvent.click(canvasElement)
        await waitFor(() => expect(notification).not.toBeInTheDocument())
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        expect(dataWrites()[0]).toEqual({
            method: "update",
            resource: "sequent_backend_ballot_style",
            params: expect.objectContaining({
                id: northBallot.id,
                data: expect.objectContaining({area_id: STORY_IDS.secondArea}),
            }),
        })
    },
}

export const UndoTheAreaChange: Story = {
    // Saving returns to the empty list route, where axe finds no defect of the form.
    parameters: {widgets: ["BallotStyleForm"], expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await areaSelect(canvasElement))
        await userEvent.click(
            await within(document.body).findByRole("option", {name: "South district"})
        )
        await userEvent.click(canvas.getByRole("button", {name: "Save"}))
        await userEvent.click(await within(document.body).findByRole("button", {name: "Undo"}))
        await waitFor(() =>
            expect(within(document.body).queryByText("Element updated")).not.toBeInTheDocument()
        )
        expect(dataWrites()).toEqual([])
    },
}

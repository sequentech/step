// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {CreateDocument} from "./CreateDocument"
import {
    DocumentLayout,
    dataWrites,
    setUpDocuments,
    type DocumentServices,
} from "./__stories__/DocumentFixture"

const meta = {
    title: "Admin/Document/CreateDocument",
    component: CreateDocument,
    args: {reads: "records", empty: false, downloadable: true},
    parameters: {
        router: {
            path: "/sequent_backend_document/create",
            initialEntries: ["/sequent_backend_document/create"],
            layout: DocumentLayout,
        },
        expectedFailure: {
            reason: "The JSON inputs grey their item counts below the contrast minimum.",
            a11y: ["color-contrast"],
        },
    },
    beforeEach: ({args}) => setUpDocuments(args),
    render: () => <CreateDocument />,
} satisfies WidgetMeta<DocumentServices>
export default meta
type Story = StoryObj<DocumentServices>

async function fillIn(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await userEvent.type(await canvas.findByRole("textbox", {name: "Name"}), "minutes.pdf")
    await userEvent.type(canvas.getByRole("textbox", {name: "Media type"}), "application/pdf")
    await userEvent.type(canvas.getByRole("spinbutton", {name: "Size"}), "2048")
    await userEvent.click(canvas.getByRole("switch", {name: "Is public"}))
    await userEvent.click(canvas.getByRole("button", {name: "Save"}))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Document creation")).toBeVisible()
        await expect(canvas.getByRole("textbox", {name: "Name"})).toHaveValue("")
        // A pristine form cannot be saved.
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(dataWrites()).toEqual([])
    },
}

export const CreateTheDocument: Story = {
    // Saving opens the new document's edit route, which the story does not render.
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await fillIn(canvasElement)
        await waitFor(() => expect(dataWrites()).toHaveLength(1))
        expect(dataWrites()[0]).toEqual({
            method: "create",
            resource: "sequent_backend_document",
            params: expect.objectContaining({
                data: expect.objectContaining({
                    name: "minutes.pdf",
                    media_type: "application/pdf",
                    size: 2048,
                    is_public: true,
                }),
            }),
        })
        await waitFor(() =>
            expect(
                within(canvasElement).getByRole("status", {name: "Current location"})
            ).toHaveTextContent("/sequent_backend_document/created-1")
        )
    },
}

export const SaveFailure: Story = {
    args: {writeError: "Synthetic document rejected"},
    play: async ({canvasElement}) => {
        await fillIn(canvasElement)
        const message = await within(document.body).findByText("Synthetic document rejected")
        await waitFor(() => expect(message).toBeVisible())
        expect(dataWrites().map(({method}) => method)).toEqual(["create"])
        await expect(
            within(canvasElement).getByRole("status", {name: "Current location"})
        ).toHaveTextContent("/sequent_backend_document/create")
    },
}

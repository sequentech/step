// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ListDocument} from "./ListDocument"
import {
    DocumentLayout,
    ROLL_ID,
    reads,
    setUpDocuments,
    type DocumentServices,
} from "./__stories__/DocumentFixture"

const meta = {
    title: "Admin/Document/ListDocument",
    component: ListDocument,
    args: {reads: "records", empty: false, downloadable: true},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        router: {
            path: "/sequent_backend_document",
            initialEntries: ["/sequent_backend_document"],
            layout: DocumentLayout,
        },
        expectedFailure: {
            reason: "React-admin row selection labels a MUI 7 span instead of its checkbox.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: ({args}) => setUpDocuments(args),
    render: () => <ListDocument />,
} satisfies WidgetMeta<DocumentServices>
export default meta
type Story = StoryObj<DocumentServices>

const row = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(name)})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const results = await row(canvasElement, "results.pdf")
        await expect(within(results).getByText("application/pdf")).toBeVisible()
        await expect(within(results).getByText("48,213")).toBeVisible()
        await expect(await row(canvasElement, "voter-roll.csv")).toBeVisible()
        // Without a document open, the side panel keeps its placeholder.
        await expect(within(canvasElement).getByText("hey")).toBeVisible()
        expect(reads("getList", "sequent_backend_document")[0].args[1]).toMatchObject({
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
        await expect(await within(canvasElement).findByText("No Documents yet.")).toBeVisible()
        expect(within(canvasElement).queryByRole("row", {name: /results/})).toBeNull()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getList", "sequent_backend_document")).toHaveLength(1))
        await expect(within(canvasElement).getByText("Documents")).toBeVisible()
        expect(within(canvasElement).queryByRole("row", {name: /results/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /results/})).toBeNull()
    },
}

export const RowShowsTheDocument: Story = {
    // The document's own route leaves the list, where axe finds no defect.
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await userEvent.click(await row(canvasElement, "voter-roll.csv"))
        const filter = JSON.stringify({election_event_id: EVENT_ID})
        await waitFor(() =>
            expect(
                within(canvasElement).getByRole("status", {name: "Current location"})
            ).toHaveTextContent(`/sequent_backend_document/${ROLL_ID}/show?filter=${filter}`)
        )
    },
}

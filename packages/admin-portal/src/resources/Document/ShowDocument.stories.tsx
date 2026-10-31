// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EVENT_ID} from "@/__stories__/AdminStoryProvider"
import {documentUrl, recordDownloads, type RecordedDownload} from "@/__stories__/downloads"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ShowDocument} from "./ShowDocument"
import {
    DocumentLayout,
    RESULTS_ID,
    graphqlCalls,
    reads,
    setUpDocuments,
    type DocumentServices,
} from "./__stories__/DocumentFixture"

let downloads: RecordedDownload[]

const meta = {
    title: "Admin/Document/ShowDocument",
    component: ShowDocument,
    args: {reads: "records", empty: false, downloadable: true},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        router: {
            path: "/sequent_backend_document/:id/show",
            initialEntries: [`/sequent_backend_document/${RESULTS_ID}/show`],
            layout: DocumentLayout,
        },
        expectedFailure: {
            reason:
                "React-admin row selection labels a MUI 7 span instead of its checkbox, " +
                "and the JSON fields grey their item counts below the contrast minimum.",
            a11y: ["aria-prohibited-attr", "color-contrast", "label"],
        },
    },
    beforeEach: async ({args}) => {
        await setUpDocuments(args)
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: () => <ShowDocument />,
} satisfies WidgetMeta<DocumentServices>
export default meta
type Story = StoryObj<DocumentServices>

const downloadButton = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("button", {name: "Download Document"})

const fetches = () => graphqlCalls().filter(({name}) => name === "FetchDocument")

export const Populated: Story = {
    parameters: {widgets: ["DocumentProperties"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await downloadButton(canvasElement)).toBeEnabled()
        await expect(canvas.getByText("Size (bytes)")).toBeVisible()
        // The list and the properties both show the report.
        expect(canvas.getAllByText("results.pdf")).toHaveLength(2)
        expect(canvas.getAllByText("48,213")).toHaveLength(2)
        expect(reads("getOne", "sequent_backend_document")[0].args[1]).toMatchObject({
            id: RESULTS_ID,
        })
        expect(fetches()).toEqual([])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(reads("getOne", "sequent_backend_document")).toHaveLength(1))
        expect(within(canvasElement).queryByRole("button", {name: "Download Document"})).toBeNull()
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
        ).toHaveTextContent(/^\/sequent_backend_document$/)
    },
}

export const DownloadTheDocument: Story = {
    parameters: {widgets: ["DocumentProperties"]},
    play: async ({canvasElement}) => {
        await userEvent.click(await downloadButton(canvasElement))
        await waitFor(() =>
            expect(downloads).toEqual([{name: "results.pdf", href: documentUrl(RESULTS_ID)}])
        )
        expect(fetches().map(({variables}) => variables)).toEqual([
            {electionEventId: EVENT_ID, documentId: RESULTS_ID},
        ])
        await expect(await downloadButton(canvasElement)).toBeEnabled()
        expect(within(canvasElement).queryByRole("alert")).toBeNull()
    },
}

export const DownloadUnavailable: Story = {
    args: {downloadable: false},
    parameters: {widgets: ["DocumentProperties"]},
    play: async ({canvasElement}) => {
        await userEvent.click(await downloadButton(canvasElement))
        await expect(await within(canvasElement).findByRole("alert")).toHaveTextContent(
            "Document download unavailable"
        )
        expect(fetches()).toHaveLength(1)
        expect(downloads).toEqual([])
    },
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {MenuList} from "@mui/material"
import {i18n} from "@sequentech/ui-core"
import {EExportFormat} from "@/types/results"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {RESULT_DOCUMENTS, documentId} from "@/resources/Tally/__stories__/TallyFixture"
import {
    documentHandlers,
    documentUrl,
    recordDownloads,
    startedTask,
    taskHandler,
    taskWidgetDefects,
    type RecordedDownload,
} from "./__stories__/DownloadFixture"
import {GeneratePDF} from "./GeneratePdf"

const PDF_ID = documentId(70)
const TASK_ID = "7a5c0000-0000-4000-8000-000000000003"

let boundary: ReturnType<typeof graphqlBoundary>
let downloads: RecordedDownload[]
/** What the render service answers: a started task, no document, or a network error. */
let service: "success" | "no-document" | "failure"

const meta = {
    title: "Admin/Tally/GeneratePDF",
    component: GeneratePDF,
    args: {
        documents: RESULT_DOCUMENTS.election,
        name: "Council",
        electionEventId: EVENT_ID,
        tallySessionId: STORY_IDS.tallySession,
        handleClose: fn(),
    },
    argTypes: {
        documentTypeToConvertFrom: {
            control: "inline-radio",
            options: [EExportFormat.HTML, EExportFormat.ALL_AREAS_HTML],
        },
        documents: {table: {disable: true}},
    },
    beforeEach: async () => {
        service = "success"
        boundary = graphqlBoundary(
            {
                ...documentHandlers({[PDF_ID]: {name: "council_results.pdf"}}),
                ...taskHandler("SUCCESS", "RENDER_DOCUMENT_PDF"),
                RenderDocumentPdf: () => {
                    if (service === "failure") throw new Error("Synthetic render service down")
                    return {
                        data: {
                            render_document_pdf: {
                                document_id: service === "success" ? PDF_ID : null,
                                task_execution: startedTask(TASK_ID, "RENDER_DOCUMENT_PDF"),
                            },
                        },
                    }
                },
            },
            {schema: true}
        )
        await boundary.ready
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    // The item is an entry of the results' export menu.
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <WidgetsContextProvider>
                <MenuList aria-label="Export results">
                    <GeneratePDF {...args} />
                </MenuList>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof GeneratePDF>
export default meta
type Story = StoryObj<typeof meta>

const pdfLabel = i18n.t("common.label.exportFormat", {item: "Council", format: "PDF"})
const taskTitle = i18n.t("tasksScreen.widget.taskTitle", {
    title: i18n.t("tasksScreen.tasksExecution.RENDER_DOCUMENT_PDF"),
})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("menuitem", {name: pdfLabel})).toBeVisible()
        expect(boundary.calls).toEqual([])
    },
}

export const RenderedPdfIsDownloaded: Story = {
    parameters: taskWidgetDefects("SUCCESS"),
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("menuitem", {name: pdfLabel}))
        expect(args.handleClose).toHaveBeenCalledTimes(1)
        await waitFor(() =>
            expect(downloads).toEqual([{name: "council_results.pdf", href: documentUrl(PDF_ID)}])
        )
        expect(boundary.calls[0]).toEqual({
            name: "RenderDocumentPdf",
            variables: {
                documentId: RESULT_DOCUMENTS.election.html,
                tallySessionId: STORY_IDS.tallySession,
                electionEventId: EVENT_ID,
            },
            headers: {"x-hasura-role": "report-read"},
        })
        await expect(await within(document.body).findByText(taskTitle)).toBeVisible()
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
    },
}

export const AllAreasSourceIsConverted: Story = {
    args: {
        documentTypeToConvertFrom: EExportFormat.ALL_AREAS_HTML,
        label: "Export All Areas Results in PDF format for 'Council'",
    },
    parameters: taskWidgetDefects("SUCCESS"),
    play: async ({canvasElement}) => {
        await userEvent.click(
            within(canvasElement).getByRole("menuitem", {
                name: "Export All Areas Results in PDF format for 'Council'",
            })
        )
        await waitFor(() => expect(downloads).toHaveLength(1))
        expect(boundary.calls[0].variables).toEqual(
            expect.objectContaining({documentId: RESULT_DOCUMENTS.election.all_areas_html})
        )
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
    },
}

export const MissingPdfMarksTheTaskFailed: Story = {
    parameters: taskWidgetDefects("FAILED"),
    beforeEach: () => {
        service = "no-document"
    },
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("menuitem", {name: pdfLabel}))
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(boundary.calls.map(({name}) => name)).toEqual(["RenderDocumentPdf"])
        expect(downloads).toEqual([])
    },
}

export const RenderFailureMarksTheTaskFailed: Story = {
    parameters: taskWidgetDefects("FAILED"),
    beforeEach: () => {
        service = "failure"
    },
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("menuitem", {name: pdfLabel}))
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(boundary.calls.map(({name}) => name)).toEqual(["RenderDocumentPdf"])
        expect(downloads).toEqual([])
    },
}

export const WithoutHtmlOffersNothing: Story = {
    args: {documents: RESULT_DOCUMENTS.electionArea},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryAllByRole("menuitem")).toEqual([])
        expect(boundary.calls).toEqual([])
    },
}

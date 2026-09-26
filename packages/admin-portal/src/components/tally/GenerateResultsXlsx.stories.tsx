// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {MenuList} from "@mui/material"
import {i18n} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {TALLY_IDS, documentId, tallyExecution} from "@/resources/Tally/__stories__/TallyFixture"
import {
    documentHandlers,
    documentUrl,
    recordDownloads,
    startedTask,
    taskHandler,
    taskWidgetDefects,
    type RecordedDownload,
} from "./__stories__/DownloadFixture"
import {GenerateResultsXlsx} from "./GenerateResultsXlsx"
import {EStoryWorkflow} from "../../../../ui-essentials/.storybook/globals"

const STORED_XLSX_ID = documentId(91)
const EXPORTED_XLSX_ID = documentId(72)
const TASK_ID = "7a5c0000-0000-4000-8000-000000000005"

let boundary: ReturnType<typeof graphqlBoundary>
let downloads: RecordedDownload[]
/** Whether the tally's execution already stores a spreadsheet of its results. */
let stored: boolean
let exportFails: boolean

const meta = {
    title: "Admin/Tally/GenerateResultsXlsx",
    component: GenerateResultsXlsx,
    args: {
        eventName: "Council",
        electionEventId: EVENT_ID,
        tallySessionId: STORY_IDS.tallySession,
        tenantId: TENANT_ID,
        resultsEventId: TALLY_IDS.resultsEvent,
        handleClose: fn(),
    },
    beforeEach: async () => {
        stored = false
        exportFails = false
        boundary = graphqlBoundary(
            {
                ...documentHandlers({
                    [STORED_XLSX_ID]: {name: "tally_results.xlsx"},
                    [EXPORTED_XLSX_ID]: {name: "exported_results.xlsx"},
                }),
                ...taskHandler("SUCCESS", "EXPORT_TALLY_RESULTS_XLSX"),
                GetTallySessionExecution: () => ({
                    data: {
                        sequent_backend_tally_session_execution: [
                            tallyExecution(EStoryWorkflow.RESULTS, {
                                documents: {
                                    sqlite: documentId(90),
                                    ...(stored ? {xlsx: STORED_XLSX_ID} : {}),
                                },
                            }),
                        ],
                    },
                }),
                ExportTallyResults: () => {
                    if (exportFails) throw new Error("Synthetic export service down")
                    return {
                        data: {
                            export_tally_results: {
                                document_id: EXPORTED_XLSX_ID,
                                error_msg: null,
                                task_execution: startedTask(TASK_ID, "EXPORT_TALLY_RESULTS_XLSX"),
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
                    <GenerateResultsXlsx {...args} />
                </MenuList>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof GenerateResultsXlsx>
export default meta
type Story = StoryObj<typeof meta>

const itemLabel = i18n.t("common.label.exportFormat", {item: "Council", format: "XLSX"})
const lookup = {
    name: "GetTallySessionExecution",
    variables: {
        tallySessionId: STORY_IDS.tallySession,
        tenantId: TENANT_ID,
        resultsEventId: TALLY_IDS.resultsEvent,
    },
    headers: {},
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("menuitem", {name: itemLabel})).toBeVisible()
        expect(boundary.calls).toEqual([])
    },
}

export const StoredSpreadsheetIsDownloaded: Story = {
    beforeEach: () => {
        stored = true
    },
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("menuitem", {name: itemLabel}))
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "tally_results.xlsx", href: documentUrl(STORED_XLSX_ID)},
            ])
        )
        expect(args.handleClose).toHaveBeenCalledTimes(1)
        expect(boundary.calls[0]).toEqual(lookup)
        expect(boundary.calls.map(({name}) => name)).not.toContain("ExportTallyResults")
    },
}

export const MissingSpreadsheetIsExported: Story = {
    parameters: taskWidgetDefects("SUCCESS"),
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("menuitem", {name: itemLabel}))
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "exported_results.xlsx", href: documentUrl(EXPORTED_XLSX_ID)},
            ])
        )
        expect(boundary.calls.slice(0, 2)).toEqual([
            lookup,
            {
                name: "ExportTallyResults",
                variables: {electionEventId: EVENT_ID, tallySessionId: STORY_IDS.tallySession},
                headers: {"x-hasura-role": "tally-results-read"},
            },
        ])
        await expect(
            await within(document.body).findByText(
                i18n.t("tasksScreen.widget.taskTitle", {
                    title: i18n.t("tasksScreen.tasksExecution.EXPORT_TALLY_RESULTS_XLSX"),
                })
            )
        ).toBeVisible()
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
    },
}

export const ExportFailureMarksTheTaskFailed: Story = {
    parameters: taskWidgetDefects("FAILED"),
    beforeEach: () => {
        exportFails = true
    },
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("menuitem", {name: itemLabel}))
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(boundary.calls.map(({name}) => name)).toEqual([
            "GetTallySessionExecution",
            "ExportTallyResults",
        ])
        expect(downloads).toEqual([])
    },
}

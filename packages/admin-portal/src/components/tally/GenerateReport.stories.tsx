// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {MenuList} from "@mui/material"
import {i18n} from "@sequentech/ui-core"
import {ETemplateType} from "@/types/templates"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {documentId} from "@/resources/Tally/__stories__/TallyFixture"
import {
    documentHandlers,
    documentUrl,
    recordDownloads,
    startedTask,
    taskHandler,
    taskWidgetDefects,
    type RecordedDownload,
} from "./__stories__/DownloadFixture"
import {GenerateReport} from "./GenerateReport"

const REPORT_ID = documentId(71)
const TASK_ID = "7a5c0000-0000-4000-8000-000000000004"

let boundary: ReturnType<typeof graphqlBoundary>
let downloads: RecordedDownload[]
/** Whether the template service answers with a started task or a network error. */
let service: "success" | "failure"

const meta = {
    title: "Admin/Tally/GenerateReport",
    component: GenerateReport,
    args: {
        electionEventId: EVENT_ID,
        electionId: STORY_IDS.election,
        tallySessionId: STORY_IDS.tallySession,
        reportType: ETemplateType.BALLOT_IMAGES,
        handleClose: fn(),
    },
    argTypes: {reportType: {control: "select", options: Object.values(ETemplateType)}},
    beforeEach: async () => {
        service = "success"
        boundary = graphqlBoundary(
            {
                ...documentHandlers({[REPORT_ID]: {name: "ballot_images.pdf"}}),
                ...taskHandler("SUCCESS", "GENERATE_REPORT"),
                GenerateTemplate: () => {
                    if (service === "failure") throw new Error("Synthetic report service down")
                    return {
                        data: {
                            generate_template: {
                                document_id: REPORT_ID,
                                task_execution: startedTask(TASK_ID, "GENERATE_REPORT"),
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
                    <GenerateReport {...args} />
                </MenuList>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof GenerateReport>
export default meta
type Story = StoryObj<typeof meta>

const itemLabel = i18n.t("tally.generateReport", {
    name: i18n.t(`template.type.${ETemplateType.BALLOT_IMAGES}`),
})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("menuitem", {name: itemLabel})).toBeVisible()
        expect(boundary.calls).toEqual([])
    },
}

export const GeneratedReportIsDownloaded: Story = {
    parameters: taskWidgetDefects("SUCCESS"),
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByRole("menuitem", {name: itemLabel}))
        expect(args.handleClose).toHaveBeenCalledTimes(1)
        await waitFor(() =>
            expect(downloads).toEqual([{name: "ballot_images.pdf", href: documentUrl(REPORT_ID)}])
        )
        // The item always asks for the ballot images template, whatever its report type.
        expect(boundary.calls[0]).toEqual({
            name: "GenerateTemplate",
            variables: {
                tallySessionId: STORY_IDS.tallySession,
                electionId: STORY_IDS.election,
                electionEventId: EVENT_ID,
                type: "BallotImages",
            },
            headers: {"x-hasura-role": "report-read"},
        })
        await expect(
            await within(document.body).findByText(
                i18n.t("tasksScreen.widget.taskTitle", {
                    title: i18n.t("tasksScreen.tasksExecution.GENERATE_REPORT"),
                })
            )
        ).toBeVisible()
        await expect(await within(document.body).findByText("SUCCESS")).toBeVisible()
    },
}

export const ServiceFailureMarksTheTaskFailed: Story = {
    parameters: taskWidgetDefects("FAILED"),
    beforeEach: () => {
        service = "failure"
    },
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("menuitem", {name: itemLabel}))
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(boundary.calls.map(({name}) => name)).toEqual(["GenerateTemplate"])
        expect(downloads).toEqual([])
    },
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {
    documentHandlers,
    documentUrl,
    exportMenuHidden,
    recordDownloads,
    startedTask,
    taskHandler,
    taskWidgetDefects,
    type RecordedDownload,
} from "./tally/__stories__/DownloadFixture"
import {MIRU_DOCUMENT_IDS, miruDocuments} from "./__stories__/MiruFixture"
import {MiruPackageDownload} from "./MiruPackageDownload"

const REPORT_ID = "d0c00000-0000-4000-8000-000000000060"
const TASK_ID = "7a5c0000-0000-4000-8000-000000000001"
/** The latest package was generated on 15 January at 12:45 UTC. */
const LATEST = "15/01/2026 12:45"
const FILE_NAME = "area__North district-__event__Council"

let boundary: ReturnType<typeof graphqlBoundary>
let downloads: RecordedDownload[]
let reportFails: boolean

const meta = {
    title: "Admin/Components/MiruPackageDownload",
    component: MiruPackageDownload,
    args: {
        documents: miruDocuments(),
        areaName: "North district",
        eventName: "Council",
        tenantId: TENANT_ID,
        electionEventId: EVENT_ID,
        electionId: STORY_IDS.election,
        tallySessionId: STORY_IDS.tallySession,
    },
    argTypes: {documents: {table: {disable: true}}},
    beforeEach: async () => {
        reportFails = false
        boundary = graphqlBoundary(
            {
                ...documentHandlers({[REPORT_ID]: {name: "transmission_report.pdf"}}),
                ...taskHandler("SUCCESS", "GENERATE_TRANSMISSION_REPORT"),
                generate_transmission_report: () => {
                    if (reportFails) throw new Error("Synthetic report service unavailable")
                    return {
                        data: {
                            generate_transmission_report: {
                                document_id: REPORT_ID,
                                encryption_policy: "unencrypted",
                                task_execution: startedTask(
                                    TASK_ID,
                                    "GENERATE_TRANSMISSION_REPORT"
                                ),
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
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <WidgetsContextProvider>
                <MiruPackageDownload {...args} />
            </WidgetsContextProvider>
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof MiruPackageDownload>
export default meta
type Story = StoryObj<typeof meta>

const label = (key: string, values?: Record<string, string>) =>
    i18n.t(`tally.transmissionPackage.actions.download.${key}`, values)

async function openMenu(canvasElement: HTMLElement) {
    await userEvent.click(within(canvasElement).getByRole("button", {name: "export election data"}))
    const menu = await within(document.body).findByRole("menu")
    await waitFor(() => expect(menu).toBeVisible())
    return within(menu)
}

async function confirmDownload() {
    const dialog = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialog).toBeVisible())
    await expect(
        within(dialog).getByText(label("dialog.description", {name: "North district"}))
    ).toBeVisible()
    expect(downloads).toEqual([])
    await userEvent.click(within(dialog).getByRole("button", {name: label("dialog.confirm")}))
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    await exportMenuHidden()
}

const operations = () => boundary.calls.map(({name}) => name)

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const menu = await openMenu(canvasElement)
        // The newest of the package's generations is offered, never the older one.
        expect(menu.getAllByRole("menuitem").map((item) => item.textContent)).toEqual([
            label("emlTitle", {date: LATEST}),
            label("transmissionPackageTitle", {date: LATEST}),
            label("transmissionReportTitle"),
        ])
        await userEvent.keyboard("{Escape}")
        await exportMenuHidden()
        expect(boundary.calls).toEqual([])
    },
}

export const DownloadEmlAfterConfirmation: Story = {
    play: async ({canvasElement}) => {
        const menu = await openMenu(canvasElement)
        await userEvent.click(menu.getByRole("menuitem", {name: label("emlTitle", {date: LATEST})}))
        await confirmDownload()
        // The date is appended after the name is sanitized, so it keeps "/" and ":".
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: `${FILE_NAME}${LATEST}.eml`, href: documentUrl(MIRU_DOCUMENT_IDS.eml)},
            ])
        )
        expect(boundary.calls.find(({name}) => name === "GetDocument")?.variables).toEqual({
            id: MIRU_DOCUMENT_IDS.eml,
            tenantId: TENANT_ID,
        })
        expect(boundary.calls.find(({name}) => name === "FetchDocument")?.variables).toEqual({
            electionEventId: EVENT_ID,
            documentId: MIRU_DOCUMENT_IDS.eml,
        })
    },
}

export const DownloadPackageAfterConfirmation: Story = {
    play: async ({canvasElement}) => {
        const menu = await openMenu(canvasElement)
        await userEvent.click(
            menu.getByRole("menuitem", {name: label("transmissionPackageTitle", {date: LATEST})})
        )
        await confirmDownload()
        await waitFor(() =>
            expect(downloads).toEqual([
                {
                    name: `${FILE_NAME}${LATEST}.zip`,
                    href: documentUrl(MIRU_DOCUMENT_IDS.allServers),
                },
            ])
        )
    },
}

export const CancelledDownloadRequestsNothing: Story = {
    play: async ({canvasElement}) => {
        const menu = await openMenu(canvasElement)
        await userEvent.click(
            menu.getByRole("menuitem", {name: label("transmissionPackageTitle", {date: LATEST})})
        )
        const dialog = await within(document.body).findByRole("dialog")
        await userEvent.click(within(dialog).getByRole("button", {name: label("dialog.cancel")}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        await exportMenuHidden()
        expect(boundary.calls).toEqual([])
        expect(downloads).toEqual([])
    },
}

export const TransmissionReportIsGeneratedAndDownloaded: Story = {
    parameters: taskWidgetDefects("SUCCESS"),
    play: async ({canvasElement}) => {
        const menu = await openMenu(canvasElement)
        await userEvent.click(menu.getByRole("menuitem", {name: label("transmissionReportTitle")}))
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "transmission_report.pdf", href: documentUrl(REPORT_ID)},
            ])
        )
        expect(boundary.calls[0]).toEqual({
            name: "generate_transmission_report",
            variables: {
                tenantId: TENANT_ID,
                electionEventId: EVENT_ID,
                electionId: STORY_IDS.election,
                tallySessionId: STORY_IDS.tallySession,
            },
            headers: {"x-hasura-role": "transmission-report-generate"},
        })
        await expect(
            await within(document.body).findByText(
                i18n.t("tasksScreen.widget.taskTitle", {
                    title: i18n.t("tasksScreen.tasksExecution.GENERATE_TRANSMISSION_REPORT"),
                })
            )
        ).toBeVisible()
        await exportMenuHidden()
    },
}

export const TransmissionReportFailureShowsTheFailedTask: Story = {
    parameters: taskWidgetDefects("FAILED"),
    beforeEach: () => {
        reportFails = true
    },
    play: async ({canvasElement}) => {
        const menu = await openMenu(canvasElement)
        await userEvent.click(menu.getByRole("menuitem", {name: label("transmissionReportTitle")}))
        await expect(await within(document.body).findByText("FAILED")).toBeVisible()
        expect(operations()).toEqual(["generate_transmission_report"])
        expect(downloads).toEqual([])
        await exportMenuHidden()
    },
}

export const WithoutPackageOnlyTheReport: Story = {
    args: {documents: []},
    play: async ({canvasElement}) => {
        const menu = await openMenu(canvasElement)
        expect(menu.getAllByRole("menuitem").map((item) => item.textContent)).toEqual([
            label("transmissionReportTitle"),
        ])
        await userEvent.keyboard("{Escape}")
        await exportMenuHidden()
    },
}

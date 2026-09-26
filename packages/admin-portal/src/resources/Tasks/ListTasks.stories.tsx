// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {STORY_IDS, eventRecord} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {
    documentHandlers,
    documentUrl,
    recordDownloads,
    type RecordedDownload,
} from "@/resources/User/__stories__/DownloadDocumentFixture"
import {ListTasks} from "./ListTasks"
import {TASK_DOCUMENT_ID, TASK_ID, taskRecords} from "./__stories__/TasksFixture"
import {
    EStoryPermissions,
    readStoryGlobals,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    reads: ReadState
    empty: boolean
    /** Whether the event record has loaded. */
    withEvent: boolean
    /** Whether the export service is unreachable. */
    exportFailure: boolean
    onViewTask: (id: string | number) => void
}

const chipContrast = "White status chip labels lack contrast on the success and warning colours."

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let downloads: RecordedDownload[]

function Fixture({withEvent, onViewTask}: Scenario) {
    const {permissions} = useStoryGlobals()
    return (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} role={permissions}>
            <ListTasks
                onViewTask={onViewTask}
                electionEventRecord={
                    withEvent ? (eventRecord() as Sequent_Backend_Election_Event) : undefined
                }
            />
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tasks/ListTasks",
    component: ListTasks,
    args: {reads: "records", empty: false, withEvent: true, exportFailure: false, onViewTask: fn()},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: `The row's view action is an icon button without an accessible name. ${chipContrast}`,
            a11y: ["button-name", "color-contrast"],
        },
    },
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {sequent_backend_tasks_execution: args.empty ? [] : taskRecords()},
            {reads: args.reads}
        )
        graphql = graphqlBoundary(
            {
                ExportTasksExecution: () => {
                    if (args.exportFailure) throw new Error("Synthetic export service unavailable")
                    return {
                        data: {
                            export_tasks_execution: {
                                error_msg: null,
                                document_id: TASK_DOCUMENT_ID,
                            },
                        },
                    }
                },
                ...documentHandlers({[TASK_DOCUMENT_ID]: {name: "export-tasks-execution.json"}}),
            },
            {schema: true}
        )
        await graphql.ready
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const taskRow = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(name)})

export const Populated: Story = {
    play: async ({canvasElement, globals}) => {
        const canvas = within(canvasElement)
        const exportRow = await taskRow(canvasElement, "Export voters")
        await expect(within(exportRow).getByText("SUCCESS")).toBeVisible()
        expect(data.calls.find(({method}) => method === "getList")?.args[1]).toMatchObject({
            filter: {election_event_id: STORY_IDS.event},
            sort: {field: "start_at", order: "DESC"},
        })
        const {permissions} = readStoryGlobals(globals)
        const admin = permissions === EStoryPermissions.ADMIN
        expect(!!canvas.queryByRole("button", {name: i18n.t("common.label.export")})).toBe(admin)
        const canView = [EStoryPermissions.ADMIN, EStoryPermissions.ADMIN_LIGHT].includes(
            permissions
        )
        expect(within(exportRow).queryAllByRole("button")).toHaveLength(canView ? 1 : 0)
    },
}

export const ViewATask: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement, args}) => {
        const row = await taskRow(canvasElement, "Export voters")
        await userEvent.click(within(row).getByRole("button"))
        expect(args.onViewTask).toHaveBeenCalledWith(TASK_ID)
    },
}

export const WithoutTaskPermissions: Story = {
    globals: {permissions: EStoryPermissions.NONE},
    parameters: {expectedFailure: {reason: chipContrast, a11y: ["color-contrast"]}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const row = await taskRow(canvasElement, "Import users")
        expect(within(row).queryByRole("button")).toBeNull()
        expect(canvas.queryByRole("button", {name: i18n.t("common.label.export")})).toBeNull()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(data.calls.map(({args}) => args[0])).toContain("sequent_backend_tasks_execution")
        )
        expect(within(canvasElement).queryByRole("row", {name: /Export voters/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /Export voters/})).toBeNull()
    },
}

export const Empty: Story = {
    args: {empty: true},
    parameters: {
        expectedFailure: {
            reason: "React-admin's empty list message is light grey below the contrast minimum.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText("No Sequent backend tasks executions yet.")
        ).toBeVisible()
        expect(within(canvasElement).queryByRole("row")).toBeNull()
    },
}

export const WaitingForTheEvent: Story = {
    args: {withEvent: false},
    parameters: {
        expectedFailure: {
            reason: "The progress indicator has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("progressbar")).toBeInTheDocument()
        expect(data.calls).toEqual([])
    },
}

async function confirmExport(canvasElement: HTMLElement) {
    await taskRow(canvasElement, "Export voters")
    await userEvent.click(
        within(canvasElement).getByRole("button", {name: i18n.t("common.label.export")})
    )
    const dialog = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialog).toBeVisible())
    expect(graphql.calls).toEqual([])
    await userEvent.click(within(dialog).getByRole("button", {name: i18n.t("common.label.export")}))
}

export const ExportAndDownload: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await confirmExport(canvasElement)
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "export-tasks-execution.json", href: documentUrl(TASK_DOCUMENT_ID)},
            ])
        )
        expect(graphql.calls[0]).toEqual({
            name: "ExportTasksExecution",
            variables: {tenantId: TENANT_ID, electionEventId: STORY_IDS.event},
            headers: {"x-hasura-role": "task-export"},
        })
        const message = await within(document.body).findByText(
            i18n.t("tasksScreen.exportTasksExecution.success")
        )
        await waitFor(() => expect(message).toBeVisible())
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const ExportFailure: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    args: {exportFailure: true},
    play: async ({canvasElement}) => {
        await confirmExport(canvasElement)
        const message = await within(document.body).findByText(
            i18n.t("tasksScreen.exportTasksExecution.error")
        )
        await waitFor(() => expect(message).toBeVisible())
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        expect(downloads).toEqual([])
    },
}

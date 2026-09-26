// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {EStoryPermissions} from "../../../ui-essentials/.storybook/globals"
import {pending} from "../../../ui-essentials/.storybook/screens"
import {ETasksExecution} from "@/types/tasksExecution"
import {
    DOCUMENT_ID,
    DOCUMENT_URL,
    TASK_ID,
    recordDownloads,
    taskLogs,
    taskRecord,
    widgetDefects,
} from "./__stories__/WidgetFixture"
import {Widget} from "./Widget"

type Props = React.ComponentProps<typeof Widget> & {
    /** What GetTaskById answers for `taskId`: the task in this status, or nothing yet. */
    execution: ETaskExecutionStatus | "pending"
}

let boundary: ReturnType<typeof graphqlBoundary>
let files: ReturnType<typeof recordDownloads>

const meta = {
    title: "Admin/Components/Widget",
    component: Widget,
    args: {
        identifier: "widget_20260115120000000",
        type: ETasksExecution.EXPORT_ELECTION_EVENT,
        status: ETaskExecutionStatus.IN_PROGRESS,
        execution: ETaskExecutionStatus.SUCCESS,
        onClose: fn(),
        onSuccess: fn(),
        onFailure: fn(),
    },
    argTypes: {
        status: {control: "select", options: Object.values(ETaskExecutionStatus)},
        execution: {
            control: "select",
            options: [...Object.values(ETaskExecutionStatus), "pending"],
        },
        type: {control: "select", options: Object.values(ETasksExecution)},
    },
    beforeEach: async ({args}) => {
        files = recordDownloads()
        boundary = graphqlBoundary(
            {
                GetTaskById: () =>
                    args.execution === "pending"
                        ? pending()
                        : {data: {sequent_backend_tasks_execution: [taskRecord(args.execution)]}},
                GetDocument: () => ({
                    data: {
                        sequent_backend_document: [{name: "council-event.zip", annotations: {}}],
                    },
                }),
                FetchDocument: () => ({data: {fetchDocument: {url: DOCUMENT_URL}}}),
            },
            {schema: true}
        )
        await boundary.ready
        return files.restore
    },
    render: ({execution: _execution, ...props}) => (
        <AdminStoryProvider boundary={boundary} role={EStoryPermissions.ADMIN}>
            <Widget {...props} />
        </AdminStoryProvider>
    ),
} satisfies Meta<Props>
export default meta
type Story = StoryObj<typeof meta>

const summaryDefects = ["button-name", "nested-interactive"] as const

const summary = (canvasElement: HTMLElement) =>
    within(canvasElement).findByText("Task: Export Election Event")

export const InProgress: Story = {
    parameters: {
        ...widgetDefects(...summaryDefects, "color-contrast", "aria-progressbar-name"),
        widgets: ["LogTable"],
    },
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await summary(canvasElement))
        await expect(canvas.getByText("IN_PROGRESS")).toBeVisible()
        await expect(canvas.getByRole("progressbar")).toBeVisible()
        await expect(await canvas.findByText("Logs")).toBeVisible()
        await expect(canvas.getByText("Task started")).toBeVisible()
        expect(canvas.queryByRole("button", {name: "View Task"})).toBeNull()
        expect(boundary.calls).toEqual([])
        expect(args.onSuccess).not.toHaveBeenCalled()
        expect(args.onFailure).not.toHaveBeenCalled()
    },
}

export const FailedShowsLogs: Story = {
    args: {status: ETaskExecutionStatus.FAILED, logs: taskLogs},
    parameters: {
        ...widgetDefects(...summaryDefects, "scrollable-region-focusable"),
        widgets: ["LogTable"],
    },
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("FAILED")).toBeVisible()
        const last = await canvas.findByText("Election event archive written")
        await waitFor(() => expect(last).toBeVisible())
        expect(getComputedStyle(last).color).toBe("rgb(139, 0, 0)")
        expect(getComputedStyle(canvas.getByText("Export started")).color).not.toBe(
            "rgb(139, 0, 0)"
        )
        expect(args.onFailure).toHaveBeenCalledTimes(1)
        expect(args.onSuccess).not.toHaveBeenCalled()
        expect(canvas.queryByRole("progressbar")).toBeNull()
    },
}

export const CloseWidget: Story = {
    parameters: widgetDefects(...summaryDefects, "color-contrast", "aria-progressbar-name"),
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await summary(canvasElement)
        const close = canvas.getByTestId("CloseIcon").closest("button")
        if (!close) throw new Error("The widget has no close button")
        await userEvent.click(close)
        expect(args.onClose).toHaveBeenCalledWith("widget_20260115120000000")
    },
}

export const TaskSucceeded: Story = {
    args: {taskId: TASK_ID},
    parameters: widgetDefects(...summaryDefects, "color-contrast", "scrollable-region-focusable"),
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("SUCCESS")).toBeVisible()
        await waitFor(() => expect(args.onSuccess).toHaveBeenCalled())
        expect(boundary.calls).toEqual([
            {name: "GetTaskById", variables: {task_id: TASK_ID}, headers: {}},
        ])
        await userEvent.click(await summary(canvasElement))
        await expect(await canvas.findByText("Voters exported")).toBeVisible()
        expect(canvas.queryByText("Task started")).toBeNull()
        await expect(canvas.getByRole("button", {name: "Download File"})).toBeEnabled()
        expect(files.downloads).toEqual([])
    },
}

export const TaskLoading: Story = {
    args: {taskId: TASK_ID, execution: "pending"},
    parameters: widgetDefects(...summaryDefects, "color-contrast", "aria-progressbar-name"),
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await waitFor(() => expect(boundary.calls.map(({name}) => name)).toEqual(["GetTaskById"]))
        await expect(canvas.getByText("IN_PROGRESS")).toBeVisible()
        await userEvent.click(await summary(canvasElement))
        await expect(await canvas.findByRole("button", {name: "View Task"})).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Download File"})).toBeNull()
        expect(args.onSuccess).not.toHaveBeenCalled()
    },
}

export const ViewTaskDetails: Story = {
    args: {taskId: TASK_ID},
    parameters: widgetDefects(...summaryDefects, "color-contrast", "scrollable-region-focusable"),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await summary(canvasElement))
        await userEvent.click(await canvas.findByRole("button", {name: "View Task"}))
        const dialog = within(
            await within(document.body).findByRole("dialog", {name: "Task Information"})
        )
        const typeRow = dialog.getByRole("row", {name: /Type/})
        await expect(within(typeRow).getByText("Export Election Event")).toBeVisible()
        await expect(dialog.getByRole("row", {name: /Executer/})).toHaveTextContent("admin")
        await userEvent.click(dialog.getByRole("button", {name: "Ok"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
    },
}

export const DownloadExportedFile: Story = {
    args: {taskId: TASK_ID},
    parameters: widgetDefects(...summaryDefects, "color-contrast", "scrollable-region-focusable"),
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await summary(canvasElement))
        const download = await canvas.findByRole("button", {name: "Download File"})
        await waitFor(() => expect(download).toBeEnabled())
        await userEvent.click(download)
        await waitFor(() =>
            expect(files.downloads).toEqual([{name: "council-event.zip", href: DOCUMENT_URL}])
        )
        const documentCalls = boundary.calls.filter(({name}) => name !== "GetTaskById")
        expect(documentCalls[0]).toEqual({
            name: "GetDocument",
            variables: {id: DOCUMENT_ID, tenantId: TENANT_ID},
            headers: {},
        })
        expect(documentCalls.filter(({name}) => name === "FetchDocument")[0]).toEqual({
            name: "FetchDocument",
            variables: {electionEventId: EVENT_ID, documentId: DOCUMENT_ID},
            headers: {},
        })
        await waitFor(() => expect(download).toBeEnabled())
    },
}

export const AutomaticDownload: Story = {
    args: {taskId: TASK_ID, automaticallyDownload: true},
    parameters: widgetDefects(...summaryDefects, "color-contrast"),
    play: async () => {
        await waitFor(() =>
            expect(files.downloads).toEqual([{name: "council-event.zip", href: DOCUMENT_URL}])
        )
    },
}

export const NoAutomaticDownloadWhileRunning: Story = {
    args: {
        taskId: TASK_ID,
        automaticallyDownload: true,
        execution: ETaskExecutionStatus.IN_PROGRESS,
    },
    parameters: widgetDefects(...summaryDefects, "color-contrast", "aria-progressbar-name"),
    play: async ({canvasElement}) => {
        await waitFor(() => expect(boundary.calls.map(({name}) => name)).toEqual(["GetTaskById"]))
        await expect(await within(canvasElement).findByRole("progressbar")).toBeVisible()
        expect(files.downloads).toEqual([])
    },
}

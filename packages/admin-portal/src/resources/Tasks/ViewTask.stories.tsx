// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {ETaskExecutionStatus, i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ETasksExecution} from "@/types/tasksExecution"
import {IPermissions} from "@/types/keycloak"
import {
    documentHandlers,
    documentUrl,
    recordDownloads,
    type RecordedDownload,
} from "@/resources/User/__stories__/DownloadDocumentFixture"
import {ViewTask} from "./ViewTask"
import {TASK_DOCUMENT_ID, TASK_ID, taskRecord} from "./__stories__/TasksFixture"
import {pending} from "../../../../ui-essentials/.storybook/screens"

interface Scenario {
    type: ETasksExecution
    status: ETaskExecutionStatus
    isModal: boolean
    /** Whether the task query answers. */
    loaded: boolean
    /** Whether the document password service answers. */
    password: "returned" | "failure"
    roles: string[]
    goBack: () => void
}

const PASSWORD = "Synthetic-PDF-Password-42"
const ALL_ROLES = [
    IPermissions.TASKS_READ,
    IPermissions.EE_TASKS_BACK_BUTTON,
    IPermissions.DOCUMENT_DOWNLOAD,
    IPermissions.DOCUMENT_PASSWORD_READ,
]

let boundary: ReturnType<typeof graphqlBoundary>
let downloads: RecordedDownload[]

const meta = {
    title: "Admin/Tasks/ViewTask",
    component: ViewTask,
    args: {
        type: ETasksExecution.EXPORT_VOTERS,
        status: ETaskExecutionStatus.SUCCESS,
        isModal: false,
        loaded: true,
        password: "returned",
        roles: ALL_ROLES,
        goBack: fn(),
    },
    argTypes: {
        type: {control: "select", options: Object.values(ETasksExecution)},
        status: {control: "select", options: Object.values(ETaskExecutionStatus)},
        password: {control: "inline-radio", options: ["returned", "failure"]},
    },
    parameters: {
        expectedFailure: {
            reason: "The white label of the task status chip lacks contrast on the status colours, and the details and logs accordions expose regions without distinct names.",
            a11y: ["color-contrast", "landmark-unique"],
        },
    },
    beforeEach: async ({args}) => {
        boundary = graphqlBoundary(
            {
                GetTaskById: () =>
                    args.loaded
                        ? {
                              data: {
                                  sequent_backend_tasks_execution: [
                                      taskRecord({type: args.type, execution_status: args.status}),
                                  ],
                              },
                          }
                        : pending(),
                GetDocumentPassword: () =>
                    args.password === "returned"
                        ? {data: {get_document_password: {password: PASSWORD}}}
                        : {errors: [new GraphQLError("Synthetic password service failure")]},
                ...documentHandlers({[TASK_DOCUMENT_ID]: {name: "voters.csv"}}),
            },
            {schema: true}
        )
        await boundary.ready
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: ({roles, goBack, isModal}) => (
        <AdminStoryProvider boundary={boundary} roles={roles}>
            <ViewTask currTaskId={TASK_ID} goBack={goBack} isModal={isModal} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const downloadButton = (element: HTMLElement) =>
    within(element).getByRole("button", {name: i18n.t("tasksScreen.widget.downloadDocument")})

export const Populated: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const details = within(await canvas.findByRole("table", {name: "task details table"}))
        await expect(
            details.getByText(i18n.t(`tasksScreen.tasksExecution.${ETasksExecution.EXPORT_VOTERS}`))
        ).toBeVisible()
        expect(details.getByText("admin")).toBeVisible()
        expect(details.getByText("1/15/2026, 12:02:00 PM")).toBeVisible()
        expect(canvas.getByText(i18n.t("tasksScreen.status", {status: "SUCCESS"}))).toBeVisible()
        expect(canvas.getByText("Exported 12 voters")).toBeVisible()
        expect(boundary.calls[0]).toEqual({
            name: "GetTaskById",
            variables: {task_id: TASK_ID},
            headers: {},
        })
        await userEvent.click(canvas.getByRole("button", {name: i18n.t("common.label.back")}))
        expect(args.goBack).toHaveBeenCalledTimes(1)
    },
}

export const Loading: Story = {
    args: {loaded: false},
    parameters: {
        expectedFailure: {
            reason: "The progress indicator has no accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        await waitFor(() => expect(boundary.calls.map(({name}) => name)).toEqual(["GetTaskById"]))
        expect(within(canvasElement).getByRole("progressbar")).toBeInTheDocument()
    },
}

export const DownloadTheResult: Story = {
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("table", {name: "task details table"})
        await userEvent.click(downloadButton(canvasElement))
        await waitFor(() =>
            expect(downloads).toEqual([{name: "voters.csv", href: documentUrl(TASK_DOCUMENT_ID)}])
        )
        expect(boundary.calls.map(({name}) => name)).not.toContain("GetDocumentPassword")
    },
}

export const RunningTaskCannotBeDownloaded: Story = {
    args: {status: ETaskExecutionStatus.IN_PROGRESS},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("table", {name: "task details table"})
        await expect(downloadButton(canvasElement)).toBeDisabled()
    },
}

export const WithoutBackPermission: Story = {
    args: {roles: [IPermissions.TASKS_READ]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("table", {name: "task details table"})
        expect(canvas.queryByRole("button", {name: i18n.t("common.label.back")})).toBeNull()
    },
}

export const ModalOkGoesBack: Story = {
    args: {isModal: true},
    play: async ({args}) => {
        const dialog = await within(document.body).findByRole("dialog")
        await waitFor(() => expect(dialog).toBeVisible())
        expect(within(dialog).getByText("Exported 12 voters")).toBeVisible()
        await userEvent.click(within(dialog).getByRole("button", {name: i18n.t("tasksScreen.ok")}))
        expect(args.goBack).toHaveBeenCalled()
    },
}

const letter = {type: ETasksExecution.VOTER_INFORMATION_LETTER}

export const VoterLetterPasswordThenDownload: Story = {
    args: letter,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(
            await canvas.findByRole("button", {
                name: i18n.t("tasksScreen.documentAccess.showPassword"),
            })
        )
        await expect(
            await canvas.findByRole("textbox", {
                name: i18n.t("tasksScreen.documentAccess.passwordLabel"),
            })
        ).toHaveValue(PASSWORD)
        expect(boundary.calls.find(({name}) => name === "GetDocumentPassword")).toEqual({
            name: "GetDocumentPassword",
            variables: {documentId: TASK_DOCUMENT_ID},
            headers: {"x-hasura-role": "document-password-read"},
        })
        await userEvent.click(downloadButton(canvasElement))
        await waitFor(() =>
            expect(downloads).toEqual([{name: "voters.csv", href: documentUrl(TASK_DOCUMENT_ID)}])
        )
    },
}

export const VoterLetterDownloadAsksForThePassword: Story = {
    args: letter,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("table", {name: "task details table"})
        await userEvent.click(downloadButton(canvasElement))
        await expect(
            await canvas.findByRole("textbox", {
                name: i18n.t("tasksScreen.documentAccess.passwordLabel"),
            })
        ).toHaveValue(PASSWORD)
        await waitFor(() => expect(downloads).toHaveLength(1))
    },
}

export const VoterLetterPasswordFailure: Story = {
    args: {...letter, password: "failure"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("table", {name: "task details table"})
        await userEvent.click(downloadButton(canvasElement))
        const message = await within(document.body).findByText(
            i18n.t("tasksScreen.documentAccess.passwordError")
        )
        await waitFor(() => expect(message).toBeVisible())
        expect(downloads).toEqual([])
        expect(canvas.queryByRole("textbox")).toBeNull()
    },
}

export const VoterLetterNeedsPasswordPermission: Story = {
    args: {...letter, roles: [IPermissions.TASKS_READ, IPermissions.DOCUMENT_DOWNLOAD]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("table", {name: "task details table"})
        expect(
            canvas.queryByText(i18n.t("tasksScreen.documentAccess.title"))
        ).not.toBeInTheDocument()
        expect(
            canvas.queryByRole("button", {name: i18n.t("tasksScreen.widget.downloadDocument")})
        ).toBeNull()
    },
}

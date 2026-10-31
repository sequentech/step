// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {GraphQLError} from "graphql"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {DEAD_LETTER_QUEUE} from "@/services/TaskQueues"
import {IPermissions} from "@/types/keycloak"
import {TaskQueues} from "./TaskQueues"
import {
    OPERATION_TASK_ID,
    OUTCOME_ROWS,
    OVERVIEW,
    SHORT_QUEUE,
    STORE_ERROR,
    SYNTAX_ERROR,
    messages,
    throughput,
} from "./__stories__/TaskQueuesFixture"

interface Scenario {
    /** The super admin's task-queue roles. */
    roles: string[]
    /** Whether the overview cannot be read. */
    overviewFails: boolean
    /** Whether Harvest refuses to queue a replay or discard. */
    operationFails: boolean
    /** Reports waiting in the short queue. */
    backlog: number
}

let graphql: ReturnType<typeof graphqlBoundary>

const ALL_ROLES = [
    IPermissions.TASK_QUEUES_READ,
    IPermissions.TASK_QUEUES_WRITE,
    IPermissions.TASK_QUEUES_QUERY,
]
const READ = {"x-hasura-role": IPermissions.TASK_QUEUES_READ}
const WRITE = {"x-hasura-role": IPermissions.TASK_QUEUES_WRITE}
const QUERY = {"x-hasura-role": IPermissions.TASK_QUEUES_QUERY}

const meta = {
    title: "Admin/Screens/TaskQueues",
    component: TaskQueues,
    args: {roles: ALL_ROLES, overviewFails: false, operationFails: false, backlog: 2},
    argTypes: {
        roles: {control: "check", options: ALL_ROLES},
    },
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary(
            {
                TaskQueuesOverview: () =>
                    args.overviewFails
                        ? {errors: [new GraphQLError("Synthetic task-queue database unavailable")]}
                        : {data: {task_queues_overview: OVERVIEW}},
                TaskQueuesThroughput: ({variables}) => ({
                    data: {
                        task_queues_throughput: throughput(
                            variables.queue,
                            variables.hours,
                            variables.bucketMinutes
                        ),
                    },
                }),
                TaskQueuesMessages: ({variables}) => ({
                    data: {
                        task_queues_messages: {
                            messages: messages(
                                variables.queue,
                                variables.state,
                                variables.before ?? null,
                                args.backlog
                            ),
                        },
                    },
                }),
                TaskQueuesDeadLetters: ({variables}) =>
                    args.operationFails
                        ? {errors: [new GraphQLError("Synthetic dead-letter operation refused")]}
                        : {
                              data: {
                                  task_queues_dead_letters: {
                                      task_id: OPERATION_TASK_ID,
                                      requested: variables.messageIds.length,
                                  },
                              },
                          },
                TaskQueuesQuery: ({variables}) => ({
                    data: {
                        task_queues_query: variables.sql.startsWith("SELECT ")
                            ? OUTCOME_ROWS
                            : {error: SYNTAX_ERROR},
                    },
                }),
            },
            {schema: true}
        )
        await graphql.ready
    },
    render: ({roles}) => (
        <AdminStoryProvider
            boundary={graphql}
            // As AuthContextProvider checks it: only the super-admin tenant passes these checks.
            auth={{
                isAuthorized: (checkSuperAdmin, _tenantId, permission) =>
                    checkSuperAdmin && [permission].flat().some((name) => roles.includes(name)),
            }}
        >
            <TaskQueues />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const calls = (name: string) => graphql.calls.filter((call) => call.name === name)

/** The overview row of a queue, found by its name button. */
async function queueRow(canvasElement: HTMLElement, queue: string) {
    const name = await within(canvasElement).findByRole("button", {name: queue})
    return within(name.closest("tr") as HTMLElement)
}

async function openDeadLetters(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await userEvent.click(await canvas.findByRole("button", {name: DEAD_LETTER_QUEUE}))
    await expect(
        await canvas.findByText(/These electoral-log events could not be processed/)
    ).toBeVisible()
    return canvas
}

export const Populated: Story = {
    parameters: {widgets: ["QueuesPanel", "Overview", "OutcomeChips", "Graphs", "Messages"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const short = await queueRow(canvasElement, SHORT_QUEUE)
        // The first queue is shown until another is chosen.
        await waitFor(() =>
            expect(short.getByRole("button", {name: SHORT_QUEUE})).toHaveAttribute(
                "aria-pressed",
                "true"
            )
        )
        for (const text of ["Succeeded 120", "Failed 2", "Expired 1", "12m 5s", "123", "18,234"]) {
            await expect(short.getByText(text)).toBeVisible()
        }
        const deadLetters = await queueRow(canvasElement, DEAD_LETTER_QUEUE)
        await expect(deadLetters.getByText("—")).toBeVisible()
        await expect(deadLetters.getByText("2h 1m")).toBeVisible()
        await expect(await canvas.findByText("Processed messages by outcome")).toBeVisible()
        await expect(canvas.getByText("Mean wait and processing time")).toBeVisible()
        await expect(await canvas.findAllByRole("row", {name: /generate_report/})).toHaveLength(2)
        // Waiting messages are only listed, never replayed or discarded.
        expect(canvas.queryByRole("button", {name: "Replay"})).toBeNull()
        expect(calls("TaskQueuesOverview")[0].headers).toEqual(READ)
        expect(calls("TaskQueuesThroughput")[0]).toMatchObject({
            variables: {queue: SHORT_QUEUE, hours: 24, bucketMinutes: 15},
            headers: READ,
        })
        expect(calls("TaskQueuesMessages")[0]).toMatchObject({
            variables: {queue: SHORT_QUEUE, state: "queued", limit: 50},
            headers: READ,
        })
    },
}

export const PeriodAndArchivedMessages: Story = {
    parameters: {widgets: ["Graphs", "Messages"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByText("Processed messages by outcome")
        await userEvent.click(canvas.getByRole("combobox", {name: /Period/}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "Last hour"}))
        await waitFor(() =>
            expect(calls("TaskQueuesThroughput").at(-1)?.variables).toEqual({
                queue: SHORT_QUEUE,
                hours: 1,
                bucketMinutes: 1,
            })
        )
        await userEvent.click(canvas.getByRole("tab", {name: "Archived"}))
        const failed = await canvas.findByRole("row", {name: /export_election_event/})
        await expect(within(failed).getByText("Failed")).toBeVisible()
        await expect(
            within(canvas.getByRole("row", {name: /render_report/})).getByText("Succeeded")
        ).toBeVisible()
        expect(calls("TaskQueuesMessages").at(-1)?.variables).toMatchObject({
            queue: SHORT_QUEUE,
            state: "archived",
        })
    },
}

export const OlderMessages: Story = {
    args: {backlog: 60},
    parameters: {widgets: ["Messages"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("row", {name: /2060/})
        await expect(canvas.getByRole("button", {name: "Newer"})).toBeDisabled()
        await userEvent.click(canvas.getByRole("button", {name: "Older"}))
        await canvas.findByRole("row", {name: /2010/})
        expect(calls("TaskQueuesMessages").at(-1)?.variables).toMatchObject({before: "2011"})
        // The last page is not full, so nothing is older.
        await expect(canvas.getByRole("button", {name: "Older"})).toBeDisabled()
        await userEvent.click(canvas.getByRole("button", {name: "Newest"}))
        await canvas.findByRole("row", {name: /2060/})
        await expect(canvas.getByRole("button", {name: "Newest"})).toBeDisabled()
    },
}

export const ReplayDeadLetters: Story = {
    parameters: {widgets: ["Messages", "ConfirmDialog"]},
    play: async ({canvasElement}) => {
        const canvas = await openDeadLetters(canvasElement)
        await expect(
            await canvas.findByText("No messages of this queue were processed in this period.")
        ).toBeVisible()
        const vote = await canvas.findByRole("row", {name: /CAST_VOTE/})
        await expect(within(vote).getByText(STORE_ERROR)).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Replay"})).toBeDisabled()
        await userEvent.click(within(vote).getByRole("checkbox"))
        await userEvent.click(canvas.getByRole("button", {name: "Replay"}))
        const dialog = within(
            await within(document.body).findByRole("dialog", {name: "Replay events?"})
        )
        await waitFor(() =>
            expect(
                dialog.getByText(
                    "1 event will be sent back to the electoral-log event queue. If it fails again, it returns here."
                )
            ).toBeVisible()
        )
        const listed = calls("TaskQueuesMessages").length
        await userEvent.click(dialog.getByRole("button", {name: "Replay"}))
        await expect(
            await canvas.findByText(`Replay of 1 event queued as task ${OPERATION_TASK_ID}.`)
        ).toBeVisible()
        expect(calls("TaskQueuesDeadLetters")).toEqual([
            {
                name: "TaskQueuesDeadLetters",
                variables: {operation: "replay", messageIds: ["101"]},
                headers: WRITE,
            },
        ])
        // The list is read again once the worker has had time to apply the operation.
        await waitFor(() => expect(calls("TaskQueuesMessages").length).toBeGreaterThan(listed), {
            timeout: 5_000,
        })
    },
}

export const CancelDiscardingAll: Story = {
    parameters: {widgets: ["Messages", "ConfirmDialog"]},
    play: async ({canvasElement}) => {
        const canvas = await openDeadLetters(canvasElement)
        await canvas.findByRole("row", {name: /LOGIN/})
        await userEvent.click(canvas.getByRole("checkbox", {name: "Select all rows"}))
        await userEvent.click(canvas.getByRole("button", {name: "Discard"}))
        const dialog = await within(document.body).findByRole("dialog", {
            name: "Discard events?",
        })
        await waitFor(() =>
            expect(
                within(dialog).getByText(
                    "2 events will be moved to the archive and not added to the electoral log."
                )
            ).toBeVisible()
        )
        await userEvent.click(within(dialog).getByRole("button", {name: "Cancel"}))
        await waitFor(() => expect(dialog).not.toBeInTheDocument())
        expect(calls("TaskQueuesDeadLetters")).toEqual([])
        await expect(canvas.getByRole("button", {name: "Discard"})).toBeEnabled()
    },
}

export const DeadLetterOperationRefused: Story = {
    args: {operationFails: true},
    parameters: {widgets: ["Messages", "ConfirmDialog"]},
    play: async ({canvasElement}) => {
        const canvas = await openDeadLetters(canvasElement)
        await userEvent.click(
            within(await canvas.findByRole("row", {name: /LOGIN/})).getByRole("checkbox")
        )
        await userEvent.click(canvas.getByRole("button", {name: "Discard"}))
        const dialog = await within(document.body).findByRole("dialog", {
            name: "Discard events?",
        })
        await userEvent.click(within(dialog).getByRole("button", {name: "Discard"}))
        await waitFor(() => expect(dialog).not.toBeInTheDocument())
        await expect(
            await canvas.findByText("Synthetic dead-letter operation refused")
        ).toBeVisible()
        expect(calls("TaskQueuesDeadLetters").map(({variables}) => variables)).toEqual([
            {operation: "discard", messageIds: ["102"]},
        ])
    },
}

export const ReadOnly: Story = {
    args: {roles: [IPermissions.TASK_QUEUES_READ]},
    parameters: {widgets: ["Messages"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: DEAD_LETTER_QUEUE}))
        await expect(
            await canvas.findByText(
                "You need the task-queues-write permission to replay or discard these events."
            )
        ).toBeVisible()
        await canvas.findByRole("row", {name: /CAST_VOTE/})
        expect(canvas.queryByRole("checkbox")).toBeNull()
        expect(canvas.queryByRole("button", {name: "Replay"})).toBeNull()
        expect(canvas.queryByRole("tab", {name: "Query"})).toBeNull()
    },
}

export const QueryTheDatabase: Story = {
    parameters: {widgets: ["QueryPanel"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("tab", {name: "Query"}))
        await expect(canvas.getByText(/Queries read the messages as they are stored/)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Run Query"}))
        await expect(await canvas.findByText("Rows: 2 · 12 ms")).toBeVisible()
        await expect(canvas.getByRole("columnheader", {name: "outcome"})).toBeVisible()
        await expect(canvas.getByRole("row", {name: /succeeded\s*120/})).toBeVisible()
        expect(calls("TaskQueuesQuery")).toEqual([
            {
                name: "TaskQueuesQuery",
                variables: {sql: expect.stringContaining("FROM pgmq.a_short_queue")},
                headers: QUERY,
            },
        ])
    },
}

export const QueryError: Story = {
    parameters: {widgets: ["QueryPanel"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("tab", {name: "Query"}))
        const sql = canvas.getByRole("textbox", {name: "Query"})
        await userEvent.clear(sql)
        await userEvent.type(sql, "SELEC 1")
        await userEvent.keyboard("{Control>}{Enter}{/Control}")
        await expect(await canvas.findByText(SYNTAX_ERROR)).toBeVisible()
        expect(canvas.queryByText(/^Rows:/)).toBeNull()
        expect(calls("TaskQueuesQuery").map(({variables}) => variables)).toEqual([{sql: "SELEC 1"}])
        await userEvent.clear(sql)
        await expect(canvas.getByRole("button", {name: "Run Query"})).toBeDisabled()
    },
}

export const OverviewUnavailable: Story = {
    args: {overviewFails: true},
    parameters: {widgets: ["QueuesPanel"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByText("Synthetic task-queue database unavailable")
        ).toBeVisible()
        expect(canvas.queryByRole("button", {name: SHORT_QUEUE})).toBeNull()
        expect(calls("TaskQueuesMessages")).toEqual([])
    },
}

export const WithoutReadPermission: Story = {
    args: {roles: []},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByText(
                "You need the task-queues-read permission of the super-admin tenant to open the task queues."
            )
        ).toBeVisible()
        expect(graphql.calls).toEqual([])
    },
}

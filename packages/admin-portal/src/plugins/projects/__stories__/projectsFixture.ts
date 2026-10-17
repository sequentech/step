// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {FetchResult, Operation as GraphqlOperation} from "@apollo/client"
import {GraphQLError} from "graphql"
import {Compatibility, PluginErrorCode, PluginState, TenantPlugin} from "../../contracts"
import {HistoryEvent, Operation, ProjectState, Saved, Task, TaskKind, TaskStatus} from "../model"

export const STORY_TIMEZONE = "Asia/Manila"
/** Story data only: the dates are illustrative, not an election body's official dates. */
export const SAMPLE_PROJECT_NAME = "2028 readiness plan (sample dates, not official COMELEC dates)"
const PROJECT_ID = "6f0c1d2e-0000-4000-8000-000000000001"
const SECOND_PROJECT_ID = "6f0c1d2e-0000-4000-8000-000000000002"
const id = (n: number) => `6f0c1d2e-0000-4000-8000-0000000001${String(n).padStart(2, "0")}`

/** Synthetic sample dates for the seven SW-F-0315 phases; not real election dates. */
const phases: Array<[string, TaskKind, string, string | null, TaskStatus]> = [
    [
        "Enrollment",
        TaskKind.Task,
        "2028-01-10T08:00:00+08:00",
        "2028-03-31T17:00:00+08:00",
        TaskStatus.InProgress,
    ],
    [
        "Election Readiness Test (ERT)",
        TaskKind.Task,
        "2028-03-06T08:00:00+08:00",
        "2028-03-10T17:00:00+08:00",
        TaskStatus.NotStarted,
    ],
    [
        "Test voting",
        TaskKind.Task,
        "2028-04-03T08:00:00+08:00",
        "2028-04-07T17:00:00+08:00",
        TaskStatus.NotStarted,
    ],
    [
        "Final Testing and Lockdown (FTL)",
        TaskKind.Task,
        "2028-04-24T08:00:00+08:00",
        "2028-04-28T17:00:00+08:00",
        TaskStatus.NotStarted,
    ],
    [
        "System lockdown",
        TaskKind.Task,
        "2028-04-29T00:00:00+08:00",
        "2028-05-08T05:00:00+08:00",
        TaskStatus.NotStarted,
    ],
    [
        "Initialization",
        TaskKind.Milestone,
        "2028-05-08T05:00:00+08:00",
        null,
        TaskStatus.NotStarted,
    ],
    [
        "Voting",
        TaskKind.Task,
        "2028-05-08T07:00:00+08:00",
        "2028-05-08T19:00:00+08:00",
        TaskStatus.NotStarted,
    ],
]

const saved = <T>(recordId: string, data: T, version = 1): Saved<T> => ({
    id: recordId,
    data,
    version,
    created_at: "2026-10-17T02:00:00Z",
    updated_at: "2026-10-17T02:42:00Z",
    updated_by: "admin",
})

export const projectsPlugin = (state = PluginState.Enabled): TenantPlugin => ({
    manifest: {
        id: "projects",
        version: "1.0.0",
        api_major: 1,
        label_key: "projects.title",
        route: "/plugins/projects",
        read_permission: "projects-read",
        permissions: ["projects-read", "projects-write", "projects-manage"],
    },
    state,
    version: 1,
    compatibility: Compatibility.Compatible,
})

type Reply = FetchResult | Promise<FetchResult>

/** The gateway's refusal, as Hasura forwards it. */
const failure = (message: string, extensions: Record<string, unknown>): Reply => ({
    errors: [new GraphQLError(message, {extensions})],
})

/** Answers the plugin gateway's GraphQL actions from memory, with versions. */
export function projectsServices({state = PluginState.Enabled}: {state?: PluginState} = {}) {
    const project = saved(PROJECT_ID, {
        name: SAMPLE_PROJECT_NAME,
        timezone: STORY_TIMEZONE,
        state: ProjectState.Active,
    })
    const second = saved(SECOND_PROJECT_ID, {
        name: "Warehouse and logistics",
        timezone: STORY_TIMEZONE,
        state: ProjectState.Active,
    })
    let tasks: Saved<Task>[] = phases.map(([title, type, start, end, status], index) =>
        saved(id(index + 1), {
            title,
            type,
            start,
            end,
            status,
            notes: `SW-F-0315 phase: ${title}.`,
            order: index,
        })
    )
    const operations: string[] = []
    const history: HistoryEvent[] = [
        {
            id: 1,
            record_id: PROJECT_ID,
            parent_id: null,
            action: "import",
            actor: "admin",
            at: "2026-10-17T02:42:00Z",
            before: null,
            after: {task_count: 7},
        },
    ]
    const reply = (data: unknown): Reply => ({data: {execute_plugin_operation: {data}}})
    const execute = (operation: string, data: Record<string, unknown>): Reply => {
        operations.push(operation)
        switch (operation) {
            case Operation.ListProjects:
                return reply({
                    items: [
                        {
                            ...project,
                            task_count: tasks.length,
                            start: tasks[0].data.start,
                            end: "2028-05-08T19:00:00+08:00",
                        },
                        {...second, task_count: 0, start: null, end: null},
                    ],
                })
            case Operation.GetProject:
                return reply(
                    data.id === PROJECT_ID ? {project, tasks} : {project: second, tasks: []}
                )
            case Operation.History:
                return reply({events: data.task_id ? [] : history})
            case Operation.SaveTask: {
                const current = tasks.find((task) => task.id === data.id)
                if (current && current.version !== data.expected_version) {
                    return failure("This record changed. Load the current version before saving", {
                        code: PluginErrorCode.Conflict,
                        current,
                    })
                }
                const next = saved(
                    String(data.id ?? id(tasks.length + 1)),
                    data.task as Task,
                    (current?.version ?? 0) + 1
                )
                tasks = current
                    ? tasks.map((task) => (task.id === next.id ? next : task))
                    : [...tasks, next]
                return reply(next)
            }
            default:
                return failure("Unknown operation", {code: PluginErrorCode.NotFound})
        }
    }
    return {
        operations,
        handlers: {
            TenantPlugins: (): Reply => ({
                data: {tenant_plugins: {data: {plugins: [projectsPlugin(state)]}}},
            }),
            ExecutePluginOperation: ({variables}: GraphqlOperation): Reply =>
                execute(String(variables.operation), variables.data as Record<string, unknown>),
        },
    }
}

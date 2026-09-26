// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The side menu's election event tree: the council event with its elections,
// contest and candidates, and the services the tree reads and changes them with.
import React, {useState, type PropsWithChildren} from "react"
import {memoryStore} from "react-admin"
import {createStore, Provider as AtomProvider} from "jotai"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import {
    STORY_IDS,
    candidateRecords,
    contestRecord,
    electionPresentation,
    electionRecord,
    eventPresentation,
    eventRecord,
    storyId,
    tenantRecord,
} from "@/__stories__/fixtures"
import {taskRecord} from "@/resources/Tasks/__stories__/TasksFixture"
import {ETasksExecution} from "@/types/tasksExecution"
import {ElectionEventTallyContextProvider} from "@/providers/ElectionEventTallyProvider"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {
    CreateElectionEventProvider,
    useCreateElectionEventStore,
} from "@/providers/CreateElectionEventContextProvider"
import type {ContestType, DynEntityType, ElectionEventType, ElectionType} from "../ElectionEvents"
import type {EStoryPermissions} from "../../../../../../ui-essentials/.storybook/globals"

export const SECOND_EVENT_ID = storyId(2, 3)
export const ARCHIVED_EVENT_ID = storyId(2, 4)
export const IMAGE_DOCUMENT_ID = storyId(6, 4)
export const DELETE_TASK_ID = storyId(5, 4)

const budgetPresentation = {
    ...eventPresentation,
    i18n: {en: {name: "Budget referendum", alias: "Budget"}},
}
const archivedPresentation = {
    ...eventPresentation,
    i18n: {en: {name: "Old council event", alias: "Old council"}},
}

/** The tenant's election event rows, as the event tree query selects them. */
export const eventRows = () => [
    eventRecord(),
    eventRecord(undefined, {id: SECOND_EVENT_ID, presentation: budgetPresentation}),
    eventRecord(undefined, {
        id: ARCHIVED_EVENT_ID,
        presentation: archivedPresentation,
        is_archived: true,
    }),
]

export const electionRows = () => [
    electionRecord(undefined, {presentation: electionPresentation("Council seats", "Seats")}),
    electionRecord(undefined, {
        id: STORY_IDS.secondElection,
        presentation: electionPresentation("Mayor election", "Mayor"),
        image_document_id: IMAGE_DOCUMENT_ID,
    }),
]

export const contestRows = () => [contestRecord()]

export const candidateRows = () => candidateRecords()

type CandidateNode = ContestType["candidates"][number]

const candidateNode = (
    candidate: ReturnType<typeof candidateRecords>[number],
    active = false
): CandidateNode => ({
    __typename: "sequent_backend_candidate",
    id: candidate.id,
    name: String(candidate.presentation?.i18n?.en?.name ?? "-"),
    tenant_id: STORY_IDS.tenant,
    election_event_id: EVENT_ID,
    election_id: STORY_IDS.election,
    contest_id: STORY_IDS.contest,
    presentation: candidate.presentation,
    ...(active ? {active} : {}),
})

function contestNode(active: boolean, candidates: CandidateNode[]): ContestType {
    const {presentation} = contestRecord()
    return {
        __typename: "sequent_backend_contest",
        id: STORY_IDS.contest,
        name: "Council members",
        tenant_id: STORY_IDS.tenant,
        election_event_id: EVENT_ID,
        election_id: STORY_IDS.election,
        max_votes: 2,
        min_votes: 0,
        winning_candidates_num: 2,
        is_encrypted: true,
        presentation,
        candidates,
        ...(active ? {active} : {}),
    }
}

function electionNode(
    record: ReturnType<typeof electionRecord>,
    active: boolean,
    contests: ContestType[]
): ElectionType {
    return {
        ...record,
        __typename: "sequent_backend_election",
        name: String(record.presentation?.i18n?.en?.name ?? "-"),
        election_event_id: EVENT_ID,
        image_document_id: record.image_document_id ?? "",
        contests,
        ...(active ? {active} : {}),
    }
}

function eventNode(
    record: ReturnType<typeof eventRecord>,
    active: boolean,
    elections: ElectionType[] = []
): ElectionEventType {
    return {
        __typename: "sequent_backend_election_event",
        id: record.id,
        name: String(record.presentation?.i18n?.en?.name ?? "-"),
        is_archived: Boolean(record.is_archived),
        presentation: record.presentation,
        elections,
        ...(active ? {active} : {}),
    }
}

/** Where in the tree the administrator is. */
export type TreeDepth = "events" | "event" | "contest" | "candidate"

/**
 * The tree the event menu builds: every active event, with the branch down to
 * the selected resource loaded.
 */
export function treeData(depth: TreeDepth = "contest"): DynEntityType {
    const [council, budget] = eventRows()
    const [seats, mayor] = electionRows()
    const candidates = candidateRows().map((candidate) =>
        candidateNode(candidate, depth === "candidate" && candidate.id === STORY_IDS.candidate)
    )
    const contests = ["contest", "candidate"].includes(depth) ? [contestNode(true, candidates)] : []
    const elections =
        depth === "events"
            ? []
            : [electionNode(seats, depth !== "event", contests), electionNode(mayor, false, [])]
    return {
        electionEvents: [
            eventNode(council, depth !== "events", elections),
            eventNode(budget, false),
        ],
    }
}

/** The archived events tab's tree. */
export const archivedTreeData = (): DynEntityType => ({
    electionEvents: [eventNode(eventRows()[2], false)],
})

export interface TreeServicesOptions {
    /** What reading the tree does. */
    reads?: ReadState
    /** Whether changing or deleting a resource fails. */
    writeFailure?: boolean
    /** How the election event deletion task ends. */
    deleteTask?: ETaskExecutionStatus
}

export interface TreeServices {
    graphql: ReturnType<typeof graphqlBoundary>
    data: ReturnType<typeof resourceBoundary>
}

const settle = <T,>(reads: ReadState, value: () => T): T | Promise<never> => {
    if (reads === "loading") return new Promise<never>(() => {})
    if (reads === "error") throw new Error("Synthetic tree service unavailable")
    return value()
}

/** Boundaries of the tree; call it in the story's beforeEach and await `graphql.ready`. */
export function treeServices({
    reads = "records",
    writeFailure = false,
    deleteTask = ETaskExecutionStatus.SUCCESS,
}: TreeServicesOptions = {}): TreeServices {
    const data = resourceBoundary(
        {
            sequent_backend_tenant: [tenantRecord],
            sequent_backend_election_event: eventRows(),
            sequent_backend_election: electionRows(),
            sequent_backend_contest: contestRows(),
            sequent_backend_candidate: candidateRows(),
            sequent_backend_document: [
                {id: IMAGE_DOCUMENT_ID, tenant_id: STORY_IDS.tenant, name: "mayor.png"},
            ],
        },
        writeFailure ? {writeError: "Synthetic write service unavailable"} : {}
    )
    const graphql = graphqlBoundary(
        {
            election_events_tree: ({variables}) =>
                settle(reads, () => ({
                    data: {
                        sequent_backend_election_event:
                            data.records.sequent_backend_election_event.filter(
                                ({is_archived}) => is_archived === variables.isArchived
                            ),
                    },
                })),
            election_tree: ({variables}) => ({
                data: {
                    sequent_backend_election: data.records.sequent_backend_election.filter(
                        ({election_event_id}) => election_event_id === variables.electionEventId
                    ),
                },
            }),
            contest_tree: ({variables}) => ({
                data: {
                    sequent_backend_contest: data.records.sequent_backend_contest.filter(
                        ({election_id}) => election_id === variables.electionId
                    ),
                },
            }),
            candidate_tree: ({variables}) => ({
                data: {
                    sequent_backend_candidate: data.records.sequent_backend_candidate.filter(
                        ({contest_id}) => contest_id === variables.contestId
                    ),
                },
            }),
            DeleteElectionEvent: ({variables}) => {
                if (writeFailure) throw new Error("Synthetic deletion service unavailable")
                const task = taskRecord({
                    id: DELETE_TASK_ID,
                    election_event_id: String(variables.electionEventId),
                    name: "Delete election event",
                    type: ETasksExecution.DELETE_ELECTION_EVENT,
                    execution_status: ETaskExecutionStatus.IN_PROGRESS,
                    annotations: {},
                })
                return {
                    data: {
                        delete_election_event: {
                            id: String(variables.electionEventId),
                            error_msg: null,
                            task_execution: task,
                        },
                    },
                }
            },
            GetTaskById: () => ({
                data: {
                    sequent_backend_tasks_execution: [
                        taskRecord({
                            id: DELETE_TASK_ID,
                            name: "Delete election event",
                            type: ETasksExecution.DELETE_ELECTION_EVENT,
                            execution_status: deleteTask,
                            annotations: {},
                        }),
                    ],
                },
            }),
        },
        {schema: true}
    )
    return {graphql, data}
}

/** Shows which of the create provider's drawers the menu opened. */
function OpenedDrawer() {
    const {createDrawer, importDrawer} = useCreateElectionEventStore()
    const opened = createDrawer ? "create" : importDrawer ? "import" : "none"
    return <output aria-label="Opened drawer">{opened}</output>
}

/** A signed-in administrator's side menu, with the providers the application wraps it in. */
export function TreeStory({
    services,
    role,
    roles,
    sidebarOpen = true,
    children,
}: PropsWithChildren<{
    services: TreeServices
    role: EStoryPermissions
    roles?: string[]
    sidebarOpen?: boolean
}>) {
    // The archived tab selection is an atom; each story starts on the active tab.
    const [atoms] = useState(() => createStore())
    return (
        <AdminStoryProvider
            boundary={services.graphql}
            dataProvider={services.data.provider}
            role={role}
            roles={roles}
            store={memoryStore({"sidebar.open": sidebarOpen})}
        >
            <AtomProvider store={atoms}>
                <ElectionEventTallyContextProvider>
                    <WidgetsContextProvider>
                        <CreateElectionEventProvider>
                            {children}
                            <OpenedDrawer />
                        </CreateElectionEventProvider>
                    </WidgetsContextProvider>
                </ElectionEventTallyContextProvider>
            </AtomProvider>
        </AdminStoryProvider>
    )
}

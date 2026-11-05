// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic task executions of the council event.
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import type {Sequent_Backend_Tasks_Execution} from "@/gql/graphql"
import {STORY_IDS, storyId, type StoryRecord} from "@/__stories__/fixtures"
import {ETasksExecution} from "@/types/tasksExecution"

/** A task row without its tenant relationship, which the task queries never select. */
export type TaskRecord = Omit<StoryRecord<Sequent_Backend_Tasks_Execution>, "tenant">

export const TASK_ID = storyId(5, 7)
export const SECOND_TASK_ID = storyId(5, 8)
export const TASK_DOCUMENT_ID = storyId(6, 7)

export function taskRecord(overrides: Partial<TaskRecord> = {}): TaskRecord {
    return {
        id: TASK_ID,
        tenant_id: STORY_IDS.tenant,
        election_event_id: STORY_IDS.event,
        name: "Export voters",
        type: ETasksExecution.EXPORT_VOTERS,
        execution_status: ETaskExecutionStatus.SUCCESS,
        executed_by_user: "admin",
        created_at: "2026-01-15T11:59:00Z",
        start_at: "2026-01-15T12:00:00Z",
        end_at: "2026-01-15T12:02:00Z",
        annotations: {document_id: TASK_DOCUMENT_ID},
        labels: {},
        logs: [
            {created_date: "2026-01-15T12:00:00Z", log_text: "Task started"},
            {created_date: "2026-01-15T12:02:00Z", log_text: "Exported 12 voters"},
        ],
        ...overrides,
    }
}

export const taskRecords = (): TaskRecord[] => [
    taskRecord(),
    taskRecord({
        id: SECOND_TASK_ID,
        name: "Import users",
        type: ETasksExecution.IMPORT_USERS,
        execution_status: ETaskExecutionStatus.FAILED,
        start_at: "2026-01-15T13:00:00Z",
        end_at: "2026-01-15T13:00:30Z",
        annotations: {},
    }),
]

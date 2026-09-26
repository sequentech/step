// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {spyOn} from "storybook/test"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import type {Sequent_Backend_Tasks_Execution} from "@/gql/graphql"
import type {IKeysCeremonyLog} from "@/services/KeyCeremony"
import {ETasksExecution} from "@/types/tasksExecution"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME, storyId, type StoryRecord} from "@/__stories__/fixtures"

export const TASK_ID = storyId(9, 1)
export const DOCUMENT_ID = storyId(9, 2)
export const DOCUMENT_URL = "https://files.admin-story.invalid/export/council-event.zip"

export const taskLogs: IKeysCeremonyLog[] = [
    {created_date: "2026-01-15T12:00:00Z", log_text: "Export started"},
    {created_date: "2026-01-15T12:00:05Z", log_text: "Voters exported"},
    {created_date: "2026-01-15T12:00:09Z", log_text: "Election event archive written"},
]

/** A task execution row; its tenant relationship is only returned when a query selects it. */
export type TaskRecord = Omit<StoryRecord<Sequent_Backend_Tasks_Execution>, "tenant">

/** A task execution as GetTaskById returns it; a finished task has an exported document. */
export function taskRecord(
    status: ETaskExecutionStatus,
    overrides: Partial<TaskRecord> = {}
): TaskRecord {
    const finished = status === ETaskExecutionStatus.SUCCESS
    return {
        id: TASK_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        name: "Export election event",
        type: ETasksExecution.EXPORT_ELECTION_EVENT,
        execution_status: status,
        executed_by_user: "admin",
        created_at: FIXED_TIME,
        start_at: FIXED_TIME,
        end_at: finished ? "2026-01-15T12:00:09Z" : null,
        logs: taskLogs,
        annotations: finished ? {document_id: DOCUMENT_ID} : {},
        labels: {},
        ...overrides,
    }
}

export interface RecordedDownload {
    name: string
    href: string
}

/** Records the links `downloadUrl` clicks instead of following them; restore it after the story. */
export function recordDownloads() {
    const downloads: RecordedDownload[] = []
    const click = spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
        this: HTMLAnchorElement
    ) {
        downloads.push({name: this.download, href: this.href})
    })
    return {downloads, restore: () => click.mockRestore()}
}

const WIDGET_DEFECTS = {
    "button-name": "the close and view icon buttons have no accessible name",
    "nested-interactive": "they sit inside the accordion's summary button",
    "color-contrast": "the status chip's white label has a contrast below 4.5:1",
    "aria-progressbar-name": "the running task's progress bar has no accessible name",
    "scrollable-region-focusable": "the scrolling log list cannot be focused with the keyboard",
} as const

/** Known accessibility defects of the widget that a story's final state shows. */
export const widgetDefects = (...rules: (keyof typeof WIDGET_DEFECTS)[]) => ({
    expectedFailure: {
        reason: `Widget: ${rules.map((rule) => WIDGET_DEFECTS[rule]).join("; ")}.`,
        a11y: rules,
    },
})

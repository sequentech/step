// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The task_queues_* actions' answers, as Harvest reads them from the environment's
// task-queue database.
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME} from "@/__stories__/fixtures"
import type {ConsoleQueryResult} from "@/services/ElectoralLogConsole"
import {
    DEAD_LETTER_QUEUE,
    MESSAGE_PAGE_SIZE,
    type MessageState,
    type MessageSummary,
    type OverviewResult,
    type ThroughputResult,
} from "@/services/TaskQueues"

export const SHORT_QUEUE = "short_queue"
export const OPERATION_TASK_ID = "77777777-7777-4777-8777-777777777771"
export const STORE_ERROR = "batch: Synthetic record could not be stored"

export const OVERVIEW: OverviewResult = {
    queues: [
        {
            queue: SHORT_QUEUE,
            ready: 3,
            running_or_scheduled: 1,
            oldest_age_secs: 725,
            total_sent: 18_234,
            last_hour: {succeeded: 120, failed: 2, expired: 1},
        },
        {
            queue: DEAD_LETTER_QUEUE,
            ready: 2,
            running_or_scheduled: 0,
            oldest_age_secs: 7_300,
            total_sent: 5,
            last_hour: {},
        },
    ],
}

/** The last two buckets of the period; the dead-letter queue processed nothing. */
export function throughput(queue: string, hours: number, bucketMinutes: number): ThroughputResult {
    const size = bucketMinutes * 60_000
    const last = Math.floor(Date.now() / size) * size
    return {
        queue,
        hours,
        bucket_minutes: bucketMinutes,
        buckets:
            queue === DEAD_LETTER_QUEUE
                ? []
                : [
                      {
                          start: new Date(last - size).toISOString(),
                          outcomes: {succeeded: 64, failed: 1},
                          mean_processing_secs: 0.84,
                          mean_wait_secs: 1.5,
                      },
                      {
                          start: new Date(last).toISOString(),
                          outcomes: {succeeded: 56, failed: 1, expired: 1},
                          mean_processing_secs: 0.91,
                          mean_wait_secs: 2.25,
                      },
                  ],
    }
}

const message = (
    fields: Pick<MessageSummary, "msg_id" | "task"> & Partial<MessageSummary>
): MessageSummary => ({
    read_count: 0,
    enqueued_at: FIXED_TIME,
    last_read_at: null,
    visible_at: FIXED_TIME,
    archived_at: null,
    task_id: `66666666-6666-4666-8666-${String(fields.msg_id).padStart(12, "0")}`,
    retries: 0,
    eta: null,
    expires: null,
    size_bytes: 1_536,
    headers: {},
    event: {},
    ...fields,
})

const deadLetter = (msg_id: number, messageType: string) =>
    message({
        msg_id,
        task: "enqueue_electoral_log_event",
        read_count: 3,
        headers: {"x-electoral-log-error": STORE_ERROR, "x-electoral-log-stage": "batch"},
        event: {tenant_id: TENANT_ID, election_event_id: EVENT_ID, message_type: messageType},
    })

const archived = (msg_id: number, task: string, outcome: string) =>
    message({
        msg_id,
        task,
        read_count: 1,
        archived_at: FIXED_TIME,
        headers: {"x-step-outcome": outcome},
    })

/**
 * A page of a queue's messages, newest first, older than `before`. The short queue
 * holds `backlog` waiting reports.
 */
export function messages(
    queue: string,
    state: MessageState,
    before: string | null,
    backlog: number
): MessageSummary[] {
    const all =
        queue === DEAD_LETTER_QUEUE
            ? state === "queued"
                ? [deadLetter(102, "LOGIN"), deadLetter(101, "CAST_VOTE")]
                : [archived(90, "enqueue_electoral_log_event", "discarded")]
            : state === "queued"
              ? Array.from({length: backlog}, (_, index) =>
                    message({msg_id: 2_000 + backlog - index, task: "generate_report"})
                )
              : [
                    archived(1_990, "render_report", "succeeded"),
                    archived(1_989, "export_election_event", "failed"),
                ]
    return all
        .filter(({msg_id}) => before === null || msg_id < Number(before))
        .slice(0, MESSAGE_PAGE_SIZE)
}

/** The default query: last hour's outcomes of the short queue. */
export const OUTCOME_ROWS: ConsoleQueryResult = {
    columns: ["outcome", "messages"],
    rows: [
        ["succeeded", 120],
        ["failed", 2],
    ],
    truncated: false,
    elapsed_ms: 12,
}

export const SYNTAX_ERROR = 'syntax error at or near "SELEC"'

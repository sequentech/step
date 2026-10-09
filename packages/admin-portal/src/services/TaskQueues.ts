// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Results of the task_queues_* actions, which Harvest serves from the environment's
// task-queue database, and how the Task Queues page shows them.

export const DEAD_LETTER_QUEUE = "electoral_log_dead_letter_queue"

// What became of a processed message, as its x-step-outcome header says. Dead letters
// that an operator discards are archived as "discarded"; archived messages without the
// header are "unknown".
export type Outcome = "succeeded" | "failed" | "expired" | "rejected" | "discarded" | "unknown"
export const OUTCOMES: Outcome[] = [
    "succeeded",
    "failed",
    "expired",
    "rejected",
    "discarded",
    "unknown",
]
export const OUTCOME_COLORS: Record<Outcome, string> = {
    succeeded: "#2e7d32",
    failed: "#d32f2f",
    expired: "#ed6c02",
    rejected: "#7b1fa2",
    discarded: "#757575",
    unknown: "#bdbdbd",
}
export const OUTCOME_HEADER = "x-step-outcome"

export const isOutcome = (value: string): value is Outcome => (OUTCOMES as string[]).includes(value)

export interface QueueOverview {
    queue: string
    ready: number
    running_or_scheduled: number
    oldest_age_secs: number | null
    total_sent: number
    last_hour: Record<string, number>
}

export interface OverviewResult {
    queues: QueueOverview[]
}

export interface ThroughputBucket {
    start: string
    outcomes: Record<string, number>
    mean_processing_secs: number | null
    mean_wait_secs: number | null
}

export interface ThroughputResult {
    queue: string
    hours: number
    bucket_minutes: number
    buckets: ThroughputBucket[]
}

export type MessageState = "queued" | "archived"
export const MESSAGE_STATES: MessageState[] = ["queued", "archived"]

export interface MessageSummary {
    msg_id: number
    read_count: number
    enqueued_at: string
    last_read_at: string | null
    visible_at: string
    archived_at: string | null
    task: string | null
    task_id: string | null
    retries: number | null
    eta: string | null
    expires: string | null
    size_bytes: number
    headers: Record<string, string>
    event: Record<string, string>
}

export interface MessagesResult {
    messages: MessageSummary[]
}

export type DeadLetterOperation = "replay" | "discard"
export const DEAD_LETTER_OPERATIONS: DeadLetterOperation[] = ["replay", "discard"]

export interface DeadLettersResult {
    task_id: string
    requested: number
}

// The periods a graph covers, each with buckets small enough to show its shape.
export type GraphPeriod = "hour" | "sixHours" | "day" | "week"
export const GRAPH_PERIODS: GraphPeriod[] = ["hour", "sixHours", "day", "week"]
export const GRAPH_RANGES: Record<GraphPeriod, {hours: number; bucketMinutes: number}> = {
    hour: {hours: 1, bucketMinutes: 1},
    sixHours: {hours: 6, bucketMinutes: 5},
    day: {hours: 24, bucketMinutes: 15},
    week: {hours: 168, bucketMinutes: 120},
}

export const MESSAGE_PAGE_SIZE = 50

/** The total of a queue's outcomes over the last hour. */
export const processedLastHour = (queue: QueueOverview): number =>
    Object.values(queue.last_hour).reduce((total, count) => total + count, 0)

/** Outcomes in their fixed order, then any the page does not know, by name. */
export const orderedOutcomes = (outcomes: Record<string, number>): Array<[string, number]> => {
    const known = OUTCOMES.filter((outcome) => outcomes[outcome] !== undefined).map(
        (outcome): [string, number] => [outcome, outcomes[outcome]]
    )
    const others = Object.keys(outcomes)
        .filter((outcome) => !isOutcome(outcome))
        .sort()
        .map((outcome): [string, number] => [outcome, outcomes[outcome]])
    return [...known, ...others]
}

export const outcomeColor = (outcome: string): string =>
    isOutcome(outcome) ? OUTCOME_COLORS[outcome] : OUTCOME_COLORS.unknown

export interface ChartSeries {
    name: string
    data: Array<[number, number | null]>
}

/**
 * The graph's series, with a point for every bucket of the period, so that a bucket
 * without messages shows as zero rather than being skipped.
 */
export const outcomeSeries = (result: ThroughputResult, now: Date): ChartSeries[] => {
    const starts = bucketStarts(result, now)
    const seen = new Set(result.buckets.flatMap((bucket) => Object.keys(bucket.outcomes)))
    const byStart = new Map(result.buckets.map((bucket) => [Date.parse(bucket.start), bucket]))
    return orderedOutcomes(Object.fromEntries(Array.from(seen, (outcome) => [outcome, 0]))).map(
        ([outcome]) => ({
            name: outcome,
            data: starts.map((start) => [start, byStart.get(start)?.outcomes[outcome] ?? 0]),
        })
    )
}

export type DurationSeries = "wait" | "processing"

/** Mean wait and processing seconds per bucket; buckets without messages have none. */
export const durationSeries = (
    result: ThroughputResult,
    now: Date
): Record<DurationSeries, ChartSeries["data"]> => {
    const starts = bucketStarts(result, now)
    const byStart = new Map(result.buckets.map((bucket) => [Date.parse(bucket.start), bucket]))
    const round = (value: number | null | undefined) =>
        value === null || value === undefined ? null : Math.round(value * 100) / 100
    return {
        wait: starts.map((start) => [start, round(byStart.get(start)?.mean_wait_secs)]),
        processing: starts.map((start) => [start, round(byStart.get(start)?.mean_processing_secs)]),
    }
}

// Harvest aligns buckets to multiples of their size since the epoch.
const bucketStarts = (result: ThroughputResult, now: Date): number[] => {
    const size = result.bucket_minutes * 60_000
    const last = Math.floor(now.getTime() / size) * size
    const first = Math.floor((now.getTime() - result.hours * 3_600_000) / size) * size
    const starts: number[] = []
    for (let start = first; start <= last; start += size) {
        starts.push(start)
    }
    return starts
}

/** Whether a series has any value to plot. */
export const hasValues = (data: ChartSeries["data"]): boolean =>
    data.some(([, value]) => value !== null)

const MAX_COUNT_TICKS = 5

/** Ticks for a count axis: whole numbers only, at most five intervals. */
export const countTicks = (series: ChartSeries[]): number => {
    const totals = new Map<number, number>()
    for (const {data} of series) {
        for (const [start, value] of data) {
            totals.set(start, (totals.get(start) ?? 0) + (value ?? 0))
        }
    }
    const highest = Math.max(0, ...Array.from(totals.values()))
    return Math.max(1, Math.min(MAX_COUNT_TICKS, Math.ceil(highest)))
}

/** An age in the largest units that keep it readable: 45s, 12m 5s, 3h 20m, 2d 4h. */
export const formatAge = (seconds: number | null): string => {
    if (seconds === null || seconds < 0) {
        return "—"
    }
    const units: Array<[string, number]> = [
        ["d", 86_400],
        ["h", 3_600],
        ["m", 60],
        ["s", 1],
    ]
    const parts: string[] = []
    let rest = Math.floor(seconds)
    for (const [unit, size] of units) {
        const amount = Math.floor(rest / size)
        if (amount > 0 || (parts.length === 0 && size === 1)) {
            parts.push(`${amount}${unit}`)
        }
        rest -= amount * size
        if (parts.length === 2) {
            break
        }
    }
    return parts.join(" ")
}

export const formatBytes = (bytes: number): string => {
    if (bytes < 1024) {
        return `${bytes} B`
    }
    if (bytes < 1024 * 1024) {
        return `${(bytes / 1024).toFixed(1)} KiB`
    }
    return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`
}

/** Operators replay or discard only waiting dead letters. */
export const managesDeadLetters = (queue: string | null, state: MessageState): boolean =>
    queue === DEAD_LETTER_QUEUE && state === "queued"

export interface RowSelection {
    type: "include" | "exclude"
    ids: ReadonlySet<string | number>
}

/** The selected message IDs: a grid's "select all" excludes rather than includes. */
export const selectedMessageIds = (selection: RowSelection, messages: MessageSummary[]): number[] =>
    messages
        .map((message) => message.msg_id)
        .filter((id) =>
            selection.type === "include" ? selection.ids.has(id) : !selection.ids.has(id)
        )

/** The ID to list older messages from, or null when the last page was not full. */
export const nextBefore = (messages: MessageSummary[], pageSize: number): string | null =>
    messages.length < pageSize ? null : String(messages[messages.length - 1].msg_id)

/** The electoral-log event a message carries, as "type · election event". */
export const eventLabel = (event: Record<string, string>): string =>
    [event.message_type, event.election_event_id].filter(Boolean).join(" · ")

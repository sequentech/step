// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import moment from "moment-timezone"

export enum TaskKind {
    Task = "task",
    Milestone = "milestone",
}
export enum TaskStatus {
    NotStarted = "not_started",
    InProgress = "in_progress",
    Done = "done",
}
export enum ProjectState {
    Active = "active",
    Archived = "archived",
}
/** Operation names of the Projects plugin API, version 1. */
export enum Operation {
    ListProjects = "list-projects",
    GetProject = "get-project",
    CreateProject = "create-project",
    SetProjectState = "set-project-state",
    SaveTask = "save-task",
    DeleteTask = "delete-task",
    PreviewImport = "preview-import",
    CommitImport = "commit-import",
    Export = "export",
    History = "history",
}
export enum Permission {
    Read = "projects-read",
    Write = "projects-write",
    Manage = "projects-manage",
}
/** Why a local date and time cannot be used in the project's timezone. */
export enum LocalTimeError {
    Invalid = "invalid",
    Skipped = "skipped",
    Repeated = "repeated",
}

export interface Task {
    title: string
    type: TaskKind
    start: string
    end: string | null
    status: TaskStatus
    notes: string
    order: number
}
export interface Project {
    name: string
    timezone: string
    state: ProjectState
}
export interface Saved<T> {
    id: string
    parent_id?: string | null
    data: T
    version: number
    created_at?: string
    updated_at: string
    updated_by: string
}
export interface ProjectSummary extends Saved<Project> {
    task_count: number
    start: string | null
    end: string | null
}
export interface ProjectDetail {
    project: Saved<Project>
    tasks: Saved<Task>[]
}
export interface HistoryEvent {
    id: number
    record_id: string | null
    parent_id: string | null
    action: string
    actor: string
    at: string
    before: {data?: Partial<Task & Project>} | null
    after: {data?: Partial<Task & Project>; task_count?: number} | null
}
export interface ImportPreview {
    valid: boolean
    rows: Array<{line: number; task: Task | null; errors: string[]}>
}

export const DAY = 86400000
export const MAX_TITLE = 200
export const MAX_NOTES = 16000
const LOCAL_FORMAT = "YYYY-MM-DDTHH:mm"
/** RFC 3339 with an explicit offset, `+00:00` rather than `Z`. */
const INSTANT_FORMAT = "YYYY-MM-DDTHH:mm:ssZ"

/** Every date is shown in the project's timezone, with its offset. */
export const formatDate = (date: string, timezone: string): string =>
    moment(date).tz(timezone).format("YYYY-MM-DD HH:mm")

export const formatOffset = (date: string, timezone: string): string =>
    moment(date).tz(timezone).format("Z")

/** The `datetime-local` value of an instant in the project's timezone. */
export const toLocalInput = (date: string, timezone: string): string =>
    moment(date).tz(timezone).format(LOCAL_FORMAT)

export type LocalTimeResult =
    | {instant: string}
    | {error: LocalTimeError.Invalid | LocalTimeError.Skipped}
    | {error: LocalTimeError.Repeated; candidates: string[]}

/**
 * Reads a wall-clock time in the project's timezone. Every offset the zone has
 * ever used is tried, and only instants that show this exact wall-clock time
 * count. A time that an offset change skips, or repeats (whatever the size of
 * the change), is reported instead of guessed; a repeated time lists its
 * instants, earliest first, for the user to choose. When `current` already
 * shows this wall-clock time it is kept, so editing other fields never
 * reinterprets a stored instant.
 */
export const fromLocalInput = (
    value: string,
    timezone: string,
    current?: string
): LocalTimeResult => {
    const zone = moment.tz.zone(timezone)
    const parts = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})$/.exec(value)
    if (!zone || !parts || !moment(value, LOCAL_FORMAT, true).isValid()) {
        return {error: LocalTimeError.Invalid}
    }
    const [year, month, day, hour, minute] = parts.slice(1).map(Number)
    if (year < 1900 || year > 2200) return {error: LocalTimeError.Invalid}
    if (current && toLocalInput(current, timezone) === value) return {instant: current}
    const wallClock = Date.UTC(year, month - 1, day, hour, minute)
    // moment-timezone stores offsets as minutes west of UTC.
    const candidates = Array.from(new Set(zone.offsets))
        .map((west) => wallClock + west * 60000)
        .filter((at) => moment.tz(at, timezone).format(LOCAL_FORMAT) === value)
    const instants = Array.from(new Set(candidates))
        .sort((a, b) => a - b)
        .map((at) => moment.tz(at, timezone).format(INSTANT_FORMAT))
    if (instants.length === 0) return {error: LocalTimeError.Skipped}
    if (instants.length > 1) return {error: LocalTimeError.Repeated, candidates: instants}
    return {instant: instants[0]}
}

/** Exact elapsed time, split into whole days, hours and minutes. */
export const duration = (task: Task): {days: number; hours: number; minutes: number} | null => {
    if (task.type === TaskKind.Milestone || !task.end) return null
    const minutes = Math.round((Date.parse(task.end) - Date.parse(task.start)) / 60000)
    return {
        days: Math.floor(minutes / 1440),
        hours: Math.floor((minutes % 1440) / 60),
        minutes: minutes % 60,
    }
}

const validInstant = (date: string): boolean =>
    /T.*(Z|[+-]\d{2}:\d{2})$/.test(date) &&
    moment(date, moment.ISO_8601, true).isValid() &&
    moment.parseZone(date).year() >= 1900 &&
    moment.parseZone(date).year() <= 2200

export const taskErrors = (task: Task): string[] => {
    const errors: string[] = []
    if (!task.title.trim() || Array.from(task.title).length > MAX_TITLE) errors.push("title")
    if (!validInstant(task.start) || (task.end !== null && !validInstant(task.end))) {
        errors.push("offset")
    }
    if (
        task.type === TaskKind.Task &&
        (!task.end || Date.parse(task.end) <= Date.parse(task.start))
    ) {
        errors.push("range")
    }
    if (task.type === TaskKind.Milestone && task.end !== null) errors.push("milestone")
    if (Array.from(task.notes).length > MAX_NOTES) errors.push("notes")
    if (!Number.isInteger(task.order) || task.order < 0 || task.order > 1000000) {
        errors.push("order")
    }
    return errors
}

/**
 * Keyboard and pointer shifts are elapsed 24-hour increments, written with the
 * project's offset at the new instant. A shift that would make the task invalid
 * leaves it unchanged.
 */
export const moveTask = (task: Task, days: number, resize: boolean, timezone: string): Task => {
    if (taskErrors(task).length) return task
    const shift = (value: string): string =>
        moment(Date.parse(value) + days * DAY)
            .tz(timezone)
            .format(INSTANT_FORMAT)
    const changed = {
        ...task,
        start: resize && task.end ? task.start : shift(task.start),
        end: task.end ? shift(task.end) : null,
    }
    return taskErrors(changed).length ? task : changed
}

export const newTask = (timezone: string, order: number): Task => {
    const start = moment().tz(timezone).startOf("day").hour(9)
    return {
        title: "",
        type: TaskKind.Task,
        start: start.format(INSTANT_FORMAT),
        end: start.clone().add(1, "day").format(INSTANT_FORMAT),
        status: TaskStatus.NotStarted,
        notes: "",
        order,
    }
}

const TASK_FIELDS: Array<keyof Task> = ["title", "type", "start", "end", "status", "notes", "order"]

/**
 * Whether a draft differs from its baseline: the saved task, or for a new task
 * the blank task it started from. Instants compare as instants.
 */
export const isDirty = (draft: Task, baseline: Task): boolean =>
    TASK_FIELDS.some((field) => {
        const a = draft[field]
        const b = baseline[field]
        if (
            (field === "start" || field === "end") &&
            typeof a === "string" &&
            typeof b === "string"
        ) {
            return Date.parse(a) !== Date.parse(b)
        }
        return a !== b
    })

export const byOrder = (a: Saved<Task>, b: Saved<Task>): number =>
    a.data.order - b.data.order || Date.parse(a.data.start) - Date.parse(b.data.start)

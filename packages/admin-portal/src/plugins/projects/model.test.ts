// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    LocalTimeError,
    TaskKind,
    TaskStatus,
    duration,
    formatDate,
    fromLocalInput,
    isDirty,
    moveTask,
    newTask,
    taskErrors,
    toLocalInput,
} from "./model"

const task = {
    title: "Prepare",
    type: TaskKind.Task,
    start: "2026-10-17T08:00:00+08:00",
    end: "2026-10-18T08:00:00+08:00",
    status: TaskStatus.NotStarted,
    notes: "keep me",
    order: 0,
}

test("dates always display in the project's timezone", () => {
    expect(formatDate("2026-10-17T00:00:00Z", "Asia/Manila")).toBe("2026-10-17 08:00")
    expect(toLocalInput("2026-10-17T00:00:00Z", "Europe/Madrid")).toBe("2026-10-17T02:00")
})

test("local inputs are read in the project's timezone and keep its offset", () => {
    expect(fromLocalInput("2026-10-17T08:00", "Asia/Manila")).toEqual({
        instant: "2026-10-17T08:00:00+08:00",
    })
    expect(fromLocalInput("2026-01-10T08:00", "Europe/Madrid")).toEqual({
        instant: "2026-01-10T08:00:00+01:00",
    })
})

test("times skipped or repeated by daylight saving are refused, not guessed", () => {
    // Clocks go forward from 02:00 to 03:00 on March 29, 2026 in Madrid.
    expect(fromLocalInput("2026-03-29T02:30", "Europe/Madrid")).toEqual({
        error: LocalTimeError.Skipped,
    })
    // And back from 03:00 to 02:00 on October 25, 2026.
    expect(fromLocalInput("2026-10-25T02:30", "Europe/Madrid")).toEqual({
        error: LocalTimeError.Repeated,
        candidates: ["2026-10-25T02:30:00+02:00", "2026-10-25T02:30:00+01:00"],
    })
    expect(fromLocalInput("2026-10-25T03:30", "Europe/Madrid")).toEqual({
        instant: "2026-10-25T03:30:00+01:00",
    })
    expect(fromLocalInput("not a date", "Asia/Manila")).toEqual({error: LocalTimeError.Invalid})
    expect(fromLocalInput("1899-12-31T23:00", "UTC")).toEqual({error: LocalTimeError.Invalid})
})

test("repeated times are found for offset changes of any size", () => {
    // Troll goes back two hours, from 03:00 to 01:00, on October 25, 2026.
    expect(fromLocalInput("2026-10-25T02:30", "Antarctica/Troll")).toEqual({
        error: LocalTimeError.Repeated,
        candidates: ["2026-10-25T02:30:00+02:00", "2026-10-25T02:30:00+00:00"],
    })
    expect(fromLocalInput("2026-10-25T03:30", "Antarctica/Troll")).toEqual({
        instant: "2026-10-25T03:30:00+00:00",
    })
    // Casey went back three hours, from 04:00 to 01:00, on March 11, 2018.
    expect(fromLocalInput("2018-03-11T02:00", "Antarctica/Casey")).toEqual({
        error: LocalTimeError.Repeated,
        candidates: ["2018-03-11T02:00:00+11:00", "2018-03-11T02:00:00+08:00"],
    })
    // Kwajalein crossed the date line westwards in 1969, repeating 23 hours.
    expect(fromLocalInput("1969-09-30T12:00", "Pacific/Kwajalein")).toEqual({
        error: LocalTimeError.Repeated,
        candidates: ["1969-09-30T12:00:00+11:00", "1969-09-30T12:00:00-12:00"],
    })
})

test("a stored instant at a repeated time is kept, not reinterpreted", () => {
    const second = "2026-10-25T02:30:00+01:00"
    expect(toLocalInput(second, "Europe/Madrid")).toBe("2026-10-25T02:30")
    expect(fromLocalInput("2026-10-25T02:30", "Europe/Madrid", second)).toEqual({instant: second})
    expect(fromLocalInput("2026-10-25T02:45", "Europe/Madrid", second)).toEqual({
        error: LocalTimeError.Repeated,
        candidates: ["2026-10-25T02:45:00+02:00", "2026-10-25T02:45:00+01:00"],
    })
})

test("moving changes both instants and preserves the rest of an unsaved task", () => {
    const moved = moveTask({...task, title: "Unsaved"}, 1, false, "Asia/Manila")
    expect(moved.start).toBe("2026-10-18T08:00:00+08:00")
    expect(moved.end).toBe("2026-10-19T08:00:00+08:00")
    expect(moved.title).toBe("Unsaved")
    expect(moved.notes).toBe("keep me")
})

test("a move across a daylight-saving change is 24 elapsed hours", () => {
    const madrid = {...task, start: "2026-10-24T12:00:00+02:00", end: "2026-10-24T13:00:00+02:00"}
    expect(moveTask(madrid, 1, false, "Europe/Madrid").start).toBe("2026-10-25T11:00:00+01:00")
})

test("resizing cannot invert a task; milestones move without an end", () => {
    expect(moveTask(task, -1, true, "Asia/Manila")).toEqual(task)
    expect(moveTask(task, 2, true, "Asia/Manila").end).toBe("2026-10-20T08:00:00+08:00")
    expect(
        moveTask({...task, type: TaskKind.Milestone, end: null}, 1, false, "Asia/Manila").end
    ).toBeNull()
})

test("invalid dates, absent offsets and mismatched milestone ends are rejected", () => {
    expect(taskErrors(task)).toEqual([])
    expect(taskErrors({...task, start: "2026-10-17T08:00"})).toContain("offset")
    expect(taskErrors({...task, start: "2026-02-30T08:00:00Z"})).toContain("offset")
    expect(taskErrors({...task, end: task.start})).toContain("range")
    expect(taskErrors({...task, title: " "})).toContain("title")
    expect(taskErrors({...task, title: "x".repeat(201)})).toContain("title")
    expect(taskErrors({...task, notes: "x".repeat(16001)})).toContain("notes")
    expect(taskErrors({...task, order: -1})).toContain("order")
    expect(taskErrors({...task, type: TaskKind.Milestone})).toContain("milestone")
})

test("durations are exact to the minute", () => {
    expect(duration(task)).toEqual({days: 1, hours: 0, minutes: 0})
    expect(duration({...task, end: "2026-10-17T08:30:00+08:00"})).toEqual({
        days: 0,
        hours: 0,
        minutes: 30,
    })
    expect(duration({...task, end: "2026-10-18T09:45:00+08:00"})).toEqual({
        days: 1,
        hours: 1,
        minutes: 45,
    })
    expect(duration({...task, type: TaskKind.Milestone, end: null})).toBeNull()
})

test("unsaved changes compare instants, not their spelling", () => {
    expect(isDirty({...task, start: "2026-10-17T00:00:00Z"}, task)).toBe(false)
    expect(isDirty({...task, title: "Changed"}, task)).toBe(true)
})

test("a new task is dirty after any edit, even with a blank title", () => {
    const blank = newTask("Asia/Manila", 3)
    expect(isDirty({...blank}, blank)).toBe(false)
    expect(isDirty({...blank, status: TaskStatus.Done}, blank)).toBe(true)
    expect(isDirty({...blank, type: TaskKind.Milestone, end: null}, blank)).toBe(true)
    expect(
        isDirty(moveTask({...blank, title: "x"}, 1, false, "Asia/Manila"), {...blank, title: "x"})
    ).toBe(true)
    expect(isDirty({...blank, order: 9}, blank)).toBe(true)
})

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    countTicks,
    DEAD_LETTER_QUEUE,
    durationSeries,
    eventLabel,
    formatAge,
    formatBytes,
    hasValues,
    managesDeadLetters,
    MessageSummary,
    nextBefore,
    orderedOutcomes,
    outcomeColor,
    OUTCOME_COLORS,
    outcomeSeries,
    processedLastHour,
    selectedMessageIds,
    ThroughputResult,
} from "./TaskQueues"

const message = (msg_id: number): MessageSummary => ({
    msg_id,
    read_count: 0,
    enqueued_at: "2026-10-07T18:00:00Z",
    last_read_at: null,
    visible_at: "2026-10-07T18:00:00Z",
    archived_at: null,
    task: "process_electoral_log_events_batch",
    task_id: `task-${msg_id}`,
    retries: 0,
    eta: null,
    expires: null,
    size_bytes: 512,
    headers: {},
    event: {},
})

describe("task-queue outcomes", () => {
    it("are listed in a fixed order, then unknown ones by name", () => {
        expect(
            orderedOutcomes({zeta: 1, failed: 2, succeeded: 5, alpha: 3, unknown: 1}).map(
                ([outcome]) => outcome
            )
        ).toEqual(["succeeded", "failed", "unknown", "alpha", "zeta"])
        expect(orderedOutcomes({})).toEqual([])
    })

    it("are totalled over the last hour", () => {
        expect(
            processedLastHour({
                queue: "beat",
                ready: 0,
                running_or_scheduled: 0,
                oldest_age_secs: null,
                total_sent: 10,
                last_hour: {succeeded: 7, failed: 2},
            })
        ).toBe(9)
    })

    it("have a color, with one for outcomes the page does not know", () => {
        expect(outcomeColor("failed")).toBe(OUTCOME_COLORS.failed)
        expect(outcomeColor("retried")).toBe(OUTCOME_COLORS.unknown)
    })
})

describe("task-queue graphs", () => {
    const now = new Date("2026-10-07T18:07:30Z")
    const result: ThroughputResult = {
        queue: "beat",
        hours: 1,
        bucket_minutes: 15,
        buckets: [
            {
                start: "2026-10-07T17:15:00Z",
                outcomes: {succeeded: 4, failed: 1},
                mean_processing_secs: 0.123456,
                mean_wait_secs: 2,
            },
            {
                start: "2026-10-07T18:00:00Z",
                outcomes: {succeeded: 2},
                mean_processing_secs: 1,
                mean_wait_secs: null,
            },
        ],
    }
    const at = (time: string) => Date.parse(time)

    it("have a point for every bucket of the period, empty ones at zero", () => {
        expect(outcomeSeries(result, now)).toEqual([
            {
                name: "succeeded",
                data: [
                    [at("2026-10-07T17:00:00Z"), 0],
                    [at("2026-10-07T17:15:00Z"), 4],
                    [at("2026-10-07T17:30:00Z"), 0],
                    [at("2026-10-07T17:45:00Z"), 0],
                    [at("2026-10-07T18:00:00Z"), 2],
                ],
            },
            {
                name: "failed",
                data: [
                    [at("2026-10-07T17:00:00Z"), 0],
                    [at("2026-10-07T17:15:00Z"), 1],
                    [at("2026-10-07T17:30:00Z"), 0],
                    [at("2026-10-07T17:45:00Z"), 0],
                    [at("2026-10-07T18:00:00Z"), 0],
                ],
            },
        ])
    })

    it("have no series without processed messages", () => {
        expect(outcomeSeries({...result, buckets: []}, now)).toEqual([])
    })

    it("count in whole numbers, with at most five intervals", () => {
        expect(countTicks(outcomeSeries(result, now))).toBe(5)
        expect(
            countTicks([
                {name: "discarded", data: [[1, 1]]},
                {name: "failed", data: [[2, 0]]},
            ])
        ).toBe(1)
        expect(
            countTicks([
                {name: "succeeded", data: [[1, 2]]},
                {name: "failed", data: [[1, 1]]},
            ])
        ).toBe(3)
        expect(countTicks([])).toBe(1)
    })

    it("leave durations out of buckets without them", () => {
        const {wait, processing} = durationSeries(result, now)
        expect(wait.map(([, value]) => value)).toEqual([null, 2, null, null, null])
        expect(processing.map(([, value]) => value)).toEqual([null, 0.12, null, null, 1])
        expect(hasValues(wait)).toBe(true)
        expect(hasValues(durationSeries({...result, buckets: []}, now).wait)).toBe(false)
    })
})

describe("task-queue values", () => {
    it("show ages in at most two units", () => {
        expect(formatAge(null)).toBe("—")
        expect(formatAge(-1)).toBe("—")
        expect(formatAge(0)).toBe("0s")
        expect(formatAge(45)).toBe("45s")
        expect(formatAge(725)).toBe("12m 5s")
        expect(formatAge(3 * 3600 + 20 * 60 + 9)).toBe("3h 20m")
        expect(formatAge(2 * 86_400 + 4 * 3600)).toBe("2d 4h")
        expect(formatAge(86_400)).toBe("1d")
    })

    it("show sizes in bytes, KiB or MiB", () => {
        expect(formatBytes(512)).toBe("512 B")
        expect(formatBytes(1536)).toBe("1.5 KiB")
        expect(formatBytes(3 * 1024 * 1024)).toBe("3.0 MiB")
    })

    it("label events by type and election event", () => {
        expect(eventLabel({message_type: "LOGIN", election_event_id: "event"})).toBe(
            "LOGIN · event"
        )
        expect(eventLabel({})).toBe("")
    })
})

describe("task-queue messages", () => {
    const messages = [message(9), message(8), message(7)]

    it("let operators manage only waiting dead letters", () => {
        expect(managesDeadLetters(DEAD_LETTER_QUEUE, "queued")).toBe(true)
        expect(managesDeadLetters(DEAD_LETTER_QUEUE, "archived")).toBe(false)
        expect(managesDeadLetters("beat", "queued")).toBe(false)
        expect(managesDeadLetters(null, "queued")).toBe(false)
    })

    it("are selected by inclusion or, after selecting all, by exclusion", () => {
        expect(selectedMessageIds({type: "include", ids: new Set([8, 7])}, messages)).toEqual([
            8, 7,
        ])
        expect(selectedMessageIds({type: "exclude", ids: new Set([8])}, messages)).toEqual([9, 7])
        expect(selectedMessageIds({type: "exclude", ids: new Set()}, messages)).toEqual([9, 8, 7])
        expect(selectedMessageIds({type: "include", ids: new Set([1])}, messages)).toEqual([])
    })

    it("page by the oldest ID of a full page", () => {
        expect(nextBefore(messages, 3)).toBe("7")
        expect(nextBefore(messages, 50)).toBeNull()
        expect(nextBefore([], 50)).toBeNull()
    })
})

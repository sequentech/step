/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {act, renderHook} from "@testing-library/react"
import {useSelectElectionCountdown} from "./useSelectElectionCountdown"

const NOW = new Date(2026, 0, 15, 12, 0, 0)
const EMPTY_COUNTDOWN = {
    years: 0,
    months: 0,
    weeks: 0,
    days: 0,
    hours: 0,
    minutes: 0,
    seconds: 0,
    totalSeconds: 0,
}

beforeEach(() => {
    jest.useFakeTimers()
    jest.setSystemTime(NOW)
})
afterEach(() => jest.useRealTimers())

it.each([undefined, "", "not-a-date"])("does not schedule a timer for %s", (date) => {
    const {result} = renderHook(() => useSelectElectionCountdown({date}))
    expect(result.current).toBeNull()
    expect(jest.getTimerCount()).toBe(0)
})

it("shows the remaining time immediately and stops at zero", () => {
    const date = new Date(NOW.getTime() + 2_000).toISOString()
    const {result} = renderHook(() => useSelectElectionCountdown({date}))
    expect(result.current).toMatchObject({seconds: 2, totalSeconds: 2})
    act(() => jest.advanceTimersByTime(1_000))
    expect(result.current).toMatchObject({seconds: 1, totalSeconds: 1})
    act(() => jest.advanceTimersByTime(1_000))
    expect(result.current).toEqual(EMPTY_COUNTDOWN)
    expect(jest.getTimerCount()).toBe(0)
})

it.each([0, -1_000, -86_400_000])(
    "never presents an elapsed deadline as time remaining (%s ms)",
    (offset) => {
        const date = new Date(NOW.getTime() + offset).toISOString()
        const {result} = renderHook(() => useSelectElectionCountdown({date}))
        expect(result.current).toEqual(EMPTY_COUNTDOWN)
        expect(jest.getTimerCount()).toBe(0)
    }
)

it("keeps its clock running when the parent rerenders and releases it on unmount", () => {
    const date = new Date(NOW.getTime() + 5_000).toISOString()
    const {result, rerender, unmount} = renderHook(() => useSelectElectionCountdown({date}))
    // Frequent parent updates must not keep postponing the next timer callback.
    for (let tick = 0; tick < 4; tick++) {
        act(() => jest.advanceTimersByTime(250))
        rerender()
    }
    expect(result.current).toMatchObject({totalSeconds: 4})
    expect(jest.getTimerCount()).toBe(1)
    unmount()
    expect(jest.getTimerCount()).toBe(0)
})

it("replaces the deadline and clears stale values when configuration becomes invalid", () => {
    const {result, rerender} = renderHook(({date}) => useSelectElectionCountdown({date}), {
        initialProps: {date: new Date(NOW.getTime() + 2_000).toISOString()},
    })
    rerender({date: new Date(NOW.getTime() + 90_000).toISOString()})
    expect(result.current).toMatchObject({minutes: 1, seconds: 30, totalSeconds: 90})
    expect(jest.getTimerCount()).toBe(1)
    rerender({date: "invalid"})
    expect(result.current).toBeNull()
    expect(jest.getTimerCount()).toBe(0)
})

it.each([
    [new Date(2026, 1, 27), new Date(2026, 2, 2), {years: 0, months: 0, weeks: 0, days: 3}],
    [new Date(2026, 0, 31), new Date(2026, 1, 28), {years: 0, months: 1, weeks: 0, days: 0}],
    [new Date(2024, 1, 29), new Date(2025, 1, 28), {years: 1, months: 0, weeks: 0, days: 0}],
    [
        new Date(2026, 0, 15),
        new Date(2026, 0, 24, 2, 3, 4),
        {years: 0, months: 0, weeks: 1, days: 2, hours: 2, minutes: 3, seconds: 4},
    ],
])("handles calendar boundaries from %s to %s", (start, end, expected) => {
    jest.setSystemTime(start)
    const {result} = renderHook(() => useSelectElectionCountdown({date: end.toISOString()}))
    expect(result.current).toMatchObject(expected)
    expect(result.current?.totalSeconds).toBe((end.getTime() - start.getTime()) / 1_000)
})

it("counts from the actual instant during the repeated hour at the end of daylight saving time", () => {
    // Run this suite with TZ=America/Toronto as well as UTC. Reconstructing a
    // local Date can otherwise move the second 01:30 back to the first 01:30.
    const repeatedHour = new Date("2026-11-01T01:30:00-05:00")
    jest.setSystemTime(repeatedHour)
    const date = new Date(repeatedHour.getTime() + 60_000).toISOString()
    const {result} = renderHook(() => useSelectElectionCountdown({date}))
    expect(result.current).toMatchObject({hours: 0, minutes: 1, seconds: 0, totalSeconds: 60})
})

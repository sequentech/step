/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {act, renderHook} from "@testing-library/react"
import {MONITORING_DEFAULT_REFRESH_MS} from "./types"
import {useMonitoringPolling} from "./useMonitoringPolling"

let visibility: Document["visibilityState"] = "visible"

function show(state: Document["visibilityState"]) {
    visibility = state
    act(() => {
        document.dispatchEvent(new Event("visibilitychange"))
    })
}

describe("useMonitoringPolling", () => {
    beforeAll(() => {
        Object.defineProperty(document, "visibilityState", {
            configurable: true,
            get: () => visibility,
        })
    })
    beforeEach(() => {
        jest.useFakeTimers()
        visibility = "visible"
    })
    afterEach(() => jest.useRealTimers())

    const advance = (ms: number) => act(() => void jest.advanceTimersByTime(ms))

    it("polls every 30 s while the tab is shown", () => {
        const onPoll = jest.fn()
        renderHook(() => useMonitoringPolling({active: true, onPoll}))
        expect(MONITORING_DEFAULT_REFRESH_MS).toBe(30_000)
        advance(29_999)
        expect(onPoll).not.toHaveBeenCalled()
        advance(1)
        expect(onPoll).toHaveBeenCalledTimes(1)
        advance(60_000)
        expect(onPoll).toHaveBeenCalledTimes(3)
    })

    it("pauses while the tab is hidden, and asks at once when it is shown again", () => {
        const onPoll = jest.fn()
        renderHook(() => useMonitoringPolling({active: true, onPoll}))
        show("hidden")
        advance(120_000)
        expect(onPoll).not.toHaveBeenCalled()
        show("visible")
        expect(onPoll).toHaveBeenCalledTimes(1)
        advance(30_000)
        expect(onPoll).toHaveBeenCalledTimes(2)
    })

    it("pauses while editing, without asking when the tab is shown again", () => {
        const onPoll = jest.fn()
        const {rerender} = renderHook(({active}) => useMonitoringPolling({active, onPoll}), {
            initialProps: {active: false},
        })
        advance(90_000)
        show("hidden")
        show("visible")
        expect(onPoll).not.toHaveBeenCalled()
        rerender({active: true})
        advance(30_000)
        expect(onPoll).toHaveBeenCalledTimes(1)
    })

    it("uses the latest callback without restarting the clock", () => {
        const first = jest.fn()
        const second = jest.fn()
        const {rerender} = renderHook(({onPoll}) => useMonitoringPolling({active: true, onPoll}), {
            initialProps: {onPoll: first},
        })
        advance(20_000)
        rerender({onPoll: second})
        advance(10_000)
        expect(first).not.toHaveBeenCalled()
        expect(second).toHaveBeenCalledTimes(1)
    })
})

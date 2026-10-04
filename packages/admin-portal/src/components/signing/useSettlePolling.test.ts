/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {act, renderHook} from "@testing-library/react"
import {SigningRequestStatus} from "@/lib/signing/types"
import {SETTLE_POLL_MAX_MS, settleDelay, useSettlePolling} from "./useSettlePolling"

describe("settleDelay", () => {
    it("waits about a second, then backs off to two", () => {
        expect(settleDelay(0)).toBe(1000)
        expect(settleDelay(1)).toBe(1000)
        expect(settleDelay(2)).toBe(1500)
        expect(settleDelay(3)).toBe(2000)
        expect(settleDelay(10)).toBe(2000)
    })

    it("stops once the waits would pass the cap", () => {
        let total = 0
        let attempt = 0
        for (let delay = settleDelay(attempt); delay !== null; delay = settleDelay(++attempt)) {
            total += delay
        }
        expect(total).toBeLessThanOrEqual(SETTLE_POLL_MAX_MS)
        expect(total).toBeGreaterThan(SETTLE_POLL_MAX_MS - 2000)
    })
})

describe("useSettlePolling", () => {
    beforeEach(() => jest.useFakeTimers())
    afterEach(() => jest.useRealTimers())

    const flush = async (ms: number) => {
        await act(async () => {
            await jest.advanceTimersByTimeAsync(ms)
        })
    }

    it("reloads a request with all its signatures until its action ran", async () => {
        const reload = jest.fn(async () => undefined)
        const {rerender} = renderHook(({status}) => useSettlePolling(status, true, reload), {
            initialProps: {status: SigningRequestStatus.Completed as SigningRequestStatus},
        })
        expect(reload).not.toHaveBeenCalled()
        await flush(1000)
        expect(reload).toHaveBeenCalledTimes(1)
        await flush(1000)
        expect(reload).toHaveBeenCalledTimes(2)
        await flush(1500)
        expect(reload).toHaveBeenCalledTimes(3)

        // The action ran: no more reloads.
        rerender({status: SigningRequestStatus.Executed})
        await flush(60_000)
        expect(reload).toHaveBeenCalledTimes(3)
    })

    it("keeps trying after a failed reload", async () => {
        const reload = jest.fn(async () => {
            throw new Error("offline")
        })
        renderHook(() => useSettlePolling(SigningRequestStatus.Completed, true, reload))
        await flush(1000)
        await flush(1000)
        expect(reload).toHaveBeenCalledTimes(2)
    })

    it("gives up after a few minutes", async () => {
        const reload = jest.fn(async () => undefined)
        renderHook(() => useSettlePolling(SigningRequestStatus.Completed, true, reload))
        // A second at a time, so each reload's re-render schedules the next.
        for (let elapsed = 0; elapsed < SETTLE_POLL_MAX_MS; elapsed += 1000) {
            await flush(1000)
        }
        const calls = reload.mock.calls.length
        expect(calls).toBeGreaterThan(80)
        for (let elapsed = 0; elapsed < 60_000; elapsed += 1000) {
            await flush(1000)
        }
        expect(reload).toHaveBeenCalledTimes(calls)
    })

    it.each([
        SigningRequestStatus.Waiting,
        SigningRequestStatus.Executed,
        SigningRequestStatus.Failed,
        SigningRequestStatus.Cancelled,
        SigningRequestStatus.Expired,
    ])("doesn't reload a %s request", async (status) => {
        const reload = jest.fn(async () => undefined)
        renderHook(() => useSettlePolling(status, true, reload))
        await flush(30_000)
        expect(reload).not.toHaveBeenCalled()
    })

    it("doesn't reload a closed panel, and stops when it closes", async () => {
        const reload = jest.fn(async () => undefined)
        const {rerender, unmount} = renderHook(
            ({active}) => useSettlePolling(SigningRequestStatus.Completed, active, reload),
            {initialProps: {active: false}}
        )
        await flush(5000)
        expect(reload).not.toHaveBeenCalled()
        rerender({active: true})
        await flush(1000)
        expect(reload).toHaveBeenCalledTimes(1)
        unmount()
        await flush(30_000)
        expect(reload).toHaveBeenCalledTimes(1)
    })
})

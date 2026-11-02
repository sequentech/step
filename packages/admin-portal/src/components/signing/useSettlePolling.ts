// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useRef, useState} from "react"
import {SigningRequestStatus} from "@/lib/signing/types"

/** The waits between reloads: about a second, then two. */
const SETTLE_POLL_DELAYS_MS = [1000, 1000, 1500, 2000]
/** How long a panel waits for the action to run before it stops asking. */
export const SETTLE_POLL_MAX_MS = 3 * 60_000

/**
 * The wait before reload number `attempt` (from 0), or `null` once the waits
 * would pass {@link SETTLE_POLL_MAX_MS}.
 */
export function settleDelay(attempt: number): number | null {
    const delayOf = (n: number) =>
        SETTLE_POLL_DELAYS_MS[Math.min(n, SETTLE_POLL_DELAYS_MS.length - 1)]
    let total = 0
    for (let n = 0; n <= attempt; n++) {
        total += delayOf(n)
    }
    return total > SETTLE_POLL_MAX_MS ? null : delayOf(attempt)
}

/**
 * While a request has all its signatures but its action hasn't run yet
 * (Completed: the action was dispatched), reloads it with a backoff until it
 * is Executed or Failed, for a few minutes at most. Only while `active`.
 */
export function useSettlePolling(
    status: SigningRequestStatus | undefined,
    active: boolean,
    reload: () => Promise<unknown>
): void {
    const attempt = useRef(0)
    const reloadRef = useRef(reload)
    reloadRef.current = reload
    // Counts reloads, so a reload that changed nothing (or failed) schedules the next.
    const [tick, setTick] = useState(0)
    const settling = active && status === SigningRequestStatus.Completed

    useEffect(() => {
        if (!settling) {
            attempt.current = 0
            return
        }
        const delay = settleDelay(attempt.current)
        if (delay === null) return
        let cancelled = false
        const timer = setTimeout(() => {
            attempt.current += 1
            reloadRef
                .current()
                .catch(() => undefined)
                .finally(() => {
                    if (!cancelled) setTick((n) => n + 1)
                })
        }, delay)
        return () => {
            cancelled = true
            clearTimeout(timer)
        }
    }, [settling, tick])
}

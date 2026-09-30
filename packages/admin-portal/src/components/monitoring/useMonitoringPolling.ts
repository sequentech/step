// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useRef} from "react"
import {MONITORING_DEFAULT_REFRESH_MS} from "./types"
import {usePageVisible} from "./usePageVisible"

export interface MonitoringPollingOptions {
    /** False while the dashboard is being edited. */
    active: boolean
    onPoll: () => void
    intervalMs?: number
}

/**
 * Asks for the dashboard every `intervalMs` (30 s by default) while the tab
 * is shown and nobody is editing. A tab shown again asks at once, rather than
 * showing figures that may be minutes old until the next tick.
 */
export function useMonitoringPolling({
    active,
    onPoll,
    intervalMs = MONITORING_DEFAULT_REFRESH_MS,
}: MonitoringPollingOptions) {
    const visible = usePageVisible()
    const latest = useRef(onPoll)
    latest.current = onPoll
    const wasVisible = useRef(visible)

    useEffect(() => {
        const shownAgain = visible && !wasVisible.current
        wasVisible.current = visible
        if (!active || !visible) return
        if (shownAgain) latest.current()
        const timer = window.setInterval(() => latest.current(), intervalMs)
        return () => window.clearInterval(timer)
    }, [active, visible, intervalMs])
}

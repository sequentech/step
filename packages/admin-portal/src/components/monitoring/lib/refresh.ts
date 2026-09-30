// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {MONITORING_DEFAULT_REFRESH_MS, MONITORING_MIN_REFRESH_MS} from "../types"

/**
 * How often the dashboard asks for new figures: as often as the server counts
 * them (`refresh_seconds`), never more often than every 5 s, and every 30 s
 * when the server does not say.
 */
export function monitoringRefreshMs(refreshSeconds: number | null | undefined): number {
    if (
        typeof refreshSeconds !== "number" ||
        !Number.isFinite(refreshSeconds) ||
        refreshSeconds <= 0
    ) {
        return MONITORING_DEFAULT_REFRESH_MS
    }
    return Math.max(MONITORING_MIN_REFRESH_MS, Math.round(refreshSeconds * 1000))
}

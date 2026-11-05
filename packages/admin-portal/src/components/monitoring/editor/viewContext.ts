// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {EMonitoringScopeSelector, type IMonitoringScope} from "./types"

const SELECTORS = Object.values(EMonitoringScopeSelector)
/** On an election's page its Post is fixed, and with it the Post's Region. */
const PINNED_BY_POST: ReadonlySet<string> = new Set([
    EMonitoringScopeSelector.REGION,
    EMonitoringScopeSelector.POST,
])

/**
 * The scope a preview draws for: the viewer's choices for the selectors
 * the dashboard offers, without the Post and its Region on an election's
 * page (the election pins them), and the whole event while the dashboard is unknown.
 */
export const previewScope = (
    chosen: Partial<Record<EMonitoringScopeSelector, string | null>>,
    dashboard: unknown,
    electionId: string | null | undefined
): IMonitoringScope => {
    const offered =
        dashboard && typeof dashboard === "object" && !Array.isArray(dashboard)
            ? (dashboard as {selectors?: unknown}).selectors
            : undefined
    const selectors = Array.isArray(offered) ? offered : []
    const scope: IMonitoringScope = {}
    for (const selector of SELECTORS) {
        const value = chosen[selector]
        if (!value || !selectors.includes(selector)) continue
        if (electionId && PINNED_BY_POST.has(selector)) continue
        scope[selector] = value
    }
    return scope
}

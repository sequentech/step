// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {EInitializationScope, EInitializeReportPolicy} from "@sequentech/ui-core"
import type {ILifecycleSnapshotEntry} from "@/queries/Lifecycle"

/** The newest applicable published copy and current copy both impose requirements. */
export const initializesPerCountry = (
    current: EInitializationScope | undefined,
    snapshots: readonly ILifecycleSnapshotEntry[],
    electionId: string
): boolean => {
    // The action returns newest targets first; older signed copies follow them.
    const published = snapshots.find(
        (entry) => entry.election_id === null || entry.election_id === electionId
    )
    return (
        current === EInitializationScope.POST_AND_COUNTRY ||
        published?.snapshot.policies?.initialization_scope === EInitializationScope.POST_AND_COUNTRY
    )
}

/** A required published report remains required until the approved publication changes it. */
export const effectiveInitializationReportPolicy = (
    current: EInitializeReportPolicy | undefined,
    snapshots: readonly ILifecycleSnapshotEntry[],
    electionId: string
): EInitializeReportPolicy | undefined => {
    const published = snapshots.find(
        (entry) => entry.election_id === null || entry.election_id === electionId
    )
    return current === EInitializeReportPolicy.REQUIRED ||
        published?.snapshot.initialization_report_policies?.[electionId] ===
            EInitializeReportPolicy.REQUIRED
        ? EInitializeReportPolicy.REQUIRED
        : current
}

export interface InitializationArea {
    id: string
    name?: string | null
    parent_id?: string | null
}
export interface InitializationCountriesData {
    sequent_backend_area: InitializationArea[]
    sequent_backend_area_contest: Array<{area_id: string; contest: {election_id: string} | null}>
    sequent_backend_ballot_style: Array<{area_id?: string | null}>
}

/** Match the backend: styled countries whose area or ancestor holds a Post contest. */
export const initializationCountries = (
    data: InitializationCountriesData,
    electionId: string
): InitializationArea[] => {
    const areas = new Map(data.sequent_backend_area.map((area) => [area.id, area]))
    const styled = new Set(
        data.sequent_backend_ballot_style.flatMap((style) => (style.area_id ? [style.area_id] : []))
    )
    const holdsContest = new Set(
        data.sequent_backend_area_contest
            .filter((link) => link.contest?.election_id === electionId)
            .map((link) => link.area_id)
    )
    return data.sequent_backend_area.filter((area) => {
        if (!styled.has(area.id)) return false
        let cursor: InitializationArea | undefined = area
        const seen = new Set<string>()
        let matches = false
        while (cursor) {
            if (seen.has(cursor.id)) return false
            seen.add(cursor.id)
            matches ||= holdsContest.has(cursor.id)
            if (!cursor.parent_id) return matches
            cursor = areas.get(cursor.parent_id)
            if (!cursor) return false
        }
        return false
    })
}

/** Only an explicit eligible country produces a filter; whole-Post keeps the old request. */
export const initializationAreaIds = (
    country: string,
    countries: readonly InitializationArea[]
): string[] | undefined => {
    if (!countries.length) throw new Error("No eligible countries")
    if (!country) return undefined
    if (!countries.some((area) => area.id === country))
        throw new Error("Country is not eligible for this Post")
    return [country]
}

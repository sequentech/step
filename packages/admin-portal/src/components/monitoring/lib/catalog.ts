// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EDataSource, type MonitoringWidget} from "../types"

export interface CatalogGroup {
    source: string
    widgets: MonitoringWidget[]
}

const SOURCE_ORDER: string[] = Object.values(EDataSource)

/** Lower case, without spaces or punctuation, so `sw f 0259` finds `SW-F-0259`. */
const normalize = (text: string) => text.toLowerCase().replace(/[\s\-_.:/·]+/g, "")

export function matchesSearch(
    widget: MonitoringWidget,
    search: string,
    sourceLabel: (source: string) => string = (source) => source
): boolean {
    const needle = normalize(search)
    if (!needle) return true
    return [
        widget.id,
        widget.title,
        widget.source,
        sourceLabel(widget.source),
        ...(widget.requirements ?? []),
    ]
        .map(normalize)
        .some((haystack) => haystack.includes(needle))
}

const sourceRank = (source: string) => {
    const rank = SOURCE_ORDER.indexOf(source)
    return rank === -1 ? SOURCE_ORDER.length : rank
}

/** The widget catalog: grouped by data source, searchable by requirement ID. */
export function catalogGroups(
    widgets: MonitoringWidget[],
    search: string,
    sourceLabel?: (source: string) => string
): CatalogGroup[] {
    const groups = new Map<string, MonitoringWidget[]>()
    for (const widget of widgets) {
        if (!matchesSearch(widget, search, sourceLabel)) continue
        groups.set(widget.source, [...(groups.get(widget.source) ?? []), widget])
    }
    return Array.from(groups.entries())
        .sort(([a], [b]) => sourceRank(a) - sourceRank(b) || a.localeCompare(b))
        .map(([source, members]) => ({
            source,
            widgets: [...members].sort((a, b) => a.title.localeCompare(b.title)),
        }))
}

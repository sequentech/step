// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/** The event's widgets, as the Add widget catalog lists them. */

import {EYamlParseStatus, parseYamlText} from "@/components/monitoring/lib/yamlPatch"
import type {IMonitoringEditorApi} from "./api"
import {asWidget, stringList} from "./formValues"
import {EMonitoringConfigKind, type IMonitoringDashboardDefinition} from "./types"

/** The theme a dashboard uses when it names none. */
export const DEFAULT_THEME = "default"

export interface IWidgetCatalogEntry {
    id: string
    title: string
    source: string
    requirements: string[]
}

export interface IWidgetCatalogGroup {
    source: string
    entries: IWidgetCatalogEntry[]
}

/** Every widget document of the event; one that does not parse, or was removed, is left out. */
export const loadWidgetCatalog = async (
    api: IMonitoringEditorApi
): Promise<IWidgetCatalogEntry[]> => {
    const documents = await api.listConfig()
    const widgets = documents.filter((entry) => entry.kind === EMonitoringConfigKind.WIDGET)
    const loaded = await Promise.all(
        widgets.map((entry) => api.getConfig({kind: EMonitoringConfigKind.WIDGET, key: entry.key}))
    )
    return loaded.flatMap((document) => {
        if (document.yaml === null) return []
        const parsed = parseYamlText(document.yaml)
        if (parsed.status !== EYamlParseStatus.OK) return []
        const widget = asWidget(parsed.value)
        return [
            {
                id: document.key,
                title: widget.title ?? document.key,
                source: widget.source ?? "",
                requirements: stringList(widget.requirements),
            },
        ]
    })
}

/** Entries whose title, id, source or a requirement ID contains `query`. */
export const filterCatalog = (entries: IWidgetCatalogEntry[], query: string) => {
    const needle = query.trim().toLowerCase()
    if (!needle) return entries
    return entries.filter((entry) =>
        [entry.title, entry.id, entry.source, ...entry.requirements].some((text) =>
            text.toLowerCase().includes(needle)
        )
    )
}

export const groupBySource = (entries: IWidgetCatalogEntry[]): IWidgetCatalogGroup[] => {
    const groups: IWidgetCatalogGroup[] = []
    for (const entry of entries) {
        const group = groups.find((candidate) => candidate.source === entry.source)
        if (group) group.entries.push(entry)
        else groups.push({source: entry.source, entries: [entry]})
    }
    return groups
}

const layoutOf = (value: unknown) => {
    const dashboard = (value ?? {}) as Partial<IMonitoringDashboardDefinition>
    return {
        theme: typeof dashboard.theme === "string" ? dashboard.theme : DEFAULT_THEME,
        widgets: Array.isArray(dashboard.layout) ? dashboard.layout.length : 0,
    }
}

/**
 * How many widgets `themeKey` styles: the layout items of every dashboard
 * that uses it, with the dashboard being edited counted as drafted.
 */
export const countThemeWidgets = async (
    api: IMonitoringEditorApi,
    themeKey: string,
    dashboardKey: string,
    draft: unknown
): Promise<number> => {
    const documents = await api.listConfig()
    const others = documents.filter(
        (entry) => entry.kind === EMonitoringConfigKind.DASHBOARD && entry.key !== dashboardKey
    )
    const loaded = await Promise.all(
        others.map((entry) =>
            api.getConfig({kind: EMonitoringConfigKind.DASHBOARD, key: entry.key})
        )
    )
    const values = loaded.flatMap((document) => {
        if (document.yaml === null) return []
        const parsed = parseYamlText(document.yaml)
        return parsed.status === EYamlParseStatus.OK ? [parsed.value] : []
    })
    return [draft, ...values]
        .map(layoutOf)
        .filter((dashboard) => dashboard.theme === themeKey)
        .reduce((total, dashboard) => total + dashboard.widgets, 0)
}

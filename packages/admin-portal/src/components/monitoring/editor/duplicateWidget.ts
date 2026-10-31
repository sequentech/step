// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    EYamlParseStatus,
    insertIn,
    parseYamlText,
    setIn,
} from "@/components/monitoring/lib/yamlPatch"
import type {IMonitoringEditorApi} from "./api"
import {asWidget, copyId, layoutEntries} from "./formValues"
import {
    EDuplicateResult,
    EMonitoringConfigKind,
    EMonitoringSaveChange,
    EMonitoringSaveStatus,
    type TDuplicateOutcome,
} from "./types"

export {EDuplicateResult, type TDuplicateOutcome}

const LAYOUT = ["layout"]

/** The copy's title from the original's, e.g. `Turnout (copy)`, in the viewer's language. */
export type TCopyTitle = (title: string) => string

/**
 * A widget's YAML saved as its copy: the new `id`, and a title told apart
 * from the original's, so the two do not read alike on the dashboard and in
 * the catalog. A widget without a title stays without one.
 */
export const widgetCopyYaml = (yaml: string, id: string, copyTitle: TCopyTitle): string => {
    const withId = setIn(yaml, ["id"], id)
    const parsed = parseYamlText(yaml)
    if (parsed.status !== EYamlParseStatus.OK) return withId
    const {title} = asWidget(parsed.value)
    return typeof title === "string" && title ? setIn(withId, ["title"], copyTitle(title)) : withId
}

/**
 * Duplicate, from the dashboard's widget menu: saves a copy of the widget
 * under the first free `<id>-copy…`, titled by `copyTitle`, and places it right after the widget's
 * first placement, keeping its width. The dashboard is saved against the
 * revision read, so a concurrent edit is refused rather than overwritten.
 * Rejects when a document cannot be read or the widget is not on the
 * dashboard, before anything is saved.
 */
export const duplicateWidgetOnDashboard = async (
    api: IMonitoringEditorApi,
    {
        dashboardId,
        widgetId,
        copyTitle,
    }: {dashboardId: string; widgetId: string; copyTitle: TCopyTitle}
): Promise<TDuplicateOutcome> => {
    const [stored, dashboard, widget] = await Promise.all([
        api.listConfig(),
        api.getConfig({kind: EMonitoringConfigKind.DASHBOARD, key: dashboardId}),
        api.getConfig({kind: EMonitoringConfigKind.WIDGET, key: widgetId}),
    ])
    if (dashboard.yaml === null) throw new Error(dashboardId)
    if (widget.yaml === null) throw new Error(widgetId)
    const parsed = parseYamlText(dashboard.yaml)
    if (parsed.status !== EYamlParseStatus.OK) throw new Error(dashboardId)
    const {entries} = layoutEntries(parsed.value)
    const original = entries.find((entry) => entry.item.widget === widgetId)
    if (!original) throw new Error(widgetId)

    const taken = new Set([
        ...stored
            .filter((entry) => entry.kind === EMonitoringConfigKind.WIDGET)
            .map((entry) => entry.key),
        ...entries.map((entry) => entry.item.widget),
    ])
    const id = copyId(widgetId, taken)
    const copy = await api.saveConfig({
        kind: EMonitoringConfigKind.WIDGET,
        key: id,
        yaml: widgetCopyYaml(widget.yaml, id, copyTitle),
        change: EMonitoringSaveChange.UPSERT,
    })
    if (copy.status !== EMonitoringSaveStatus.SAVED) {
        return {
            result: EDuplicateResult.NOT_SAVED,
            problem:
                copy.status === EMonitoringSaveStatus.INVALID
                    ? copy.problems[0]?.message
                    : undefined,
        }
    }

    const placed = await api.saveConfig({
        kind: EMonitoringConfigKind.DASHBOARD,
        key: dashboardId,
        yaml: insertIn(dashboard.yaml, LAYOUT, original.index + 1, {
            ...original.item,
            widget: id,
        }),
        expected_revision: dashboard.revision,
        change: EMonitoringSaveChange.UPSERT,
    })
    if (placed.status === EMonitoringSaveStatus.SAVED) return {result: EDuplicateResult.DONE, id}
    return {
        result: EDuplicateResult.NOT_PLACED,
        id,
        status: placed.status,
        ...(placed.status === EMonitoringSaveStatus.INVALID
            ? {problem: placed.problems[0]?.message}
            : {}),
    }
}

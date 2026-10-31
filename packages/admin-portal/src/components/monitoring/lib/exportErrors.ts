// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {MonitoringWidget} from "../types"
import {
    EMonitoringErrorCode,
    monitoringErrorCode,
    monitoringErrorMessage,
    monitoringProblems,
    type MonitoringRequestProblem,
} from "./errors"

/** The translation function, as much of it as this needs. */
type Translate = (key: string, values?: Record<string, unknown>) => string

/** Where Harvest puts a refused pick: `widget_selector_values.<widget>.<selector>`. */
const PICK_PATH = /^widget_selector_values\.([^.]+)\.(.+)$/

/** Problem codes Harvest reports of an export (`sequent_core::monitoring::problem::Code`). */
enum EExportProblem {
    INVALID_VALUE = "invalid_value",
    UNKNOWN_SELECTOR = "unknown_selector",
    UNKNOWN_OPTION = "unknown_option",
}

const RANGE_BOUNDS = new Set(["from", "to"])

function describe(
    problem: MonitoringRequestProblem,
    t: Translate,
    widgets: MonitoringWidget[]
): string | undefined {
    if (problem.code === EExportProblem.INVALID_VALUE && RANGE_BOUNDS.has(problem.path)) {
        return t("monitoring.export.invalidRange")
    }
    const pick = PICK_PATH.exec(problem.path)
    if (
        pick &&
        (problem.code === EExportProblem.UNKNOWN_SELECTOR ||
            problem.code === EExportProblem.UNKNOWN_OPTION)
    ) {
        const [, widgetId, name] = pick
        const widget = widgets.find((candidate) => candidate.id === widgetId)
        const selector = widget?.selectors?.[name]?.label ?? name
        const key =
            problem.code === EExportProblem.UNKNOWN_SELECTOR
                ? "monitoring.export.problems.unknownSelector"
                : "monitoring.export.problems.unknownOption"
        return t(key, {widget: widget?.title ?? widgetId, selector})
    }
    return undefined
}

/**
 * What the export dialog tells the viewer of a refused export: each problem
 * Harvest found, in the viewer's words where the view knows it, else the
 * refusal's own message with Harvest's words for the problem.
 */
export function exportFailure(error: unknown, t: Translate, widgets: MonitoringWidget[]): string {
    if (monitoringErrorCode(error) !== EMonitoringErrorCode.INVALID) {
        return t(monitoringErrorMessage(error))
    }
    const problems = monitoringProblems(error)
    const known = problems.map((problem) => describe(problem, t, widgets))
    if (problems.length && known.every(Boolean)) return known.join(" ")
    return [
        t(monitoringErrorMessage(error)),
        ...problems.map((problem, index) => known[index] ?? problem.message),
    ]
        .filter(Boolean)
        .join(" ")
}

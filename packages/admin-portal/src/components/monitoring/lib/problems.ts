// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Problems as the editor keeps them. Apart from the YAML-aware
 * `diagnostics`, so the requests that carry problems load no YAML parser.
 */

import {EMonitoringProblemSeverity, type IMonitoringProblem} from "../editor/types"

const SEVERITIES: Record<string, EMonitoringProblemSeverity> = {
    error: EMonitoringProblemSeverity.ERROR,
    warning: EMonitoringProblemSeverity.WARNING,
}

/** Problems as they arrive, from WASM (`"error"`) or Harvest (`"ERROR"`), in one shape. */
export const normalizeProblems = (raw: unknown): IMonitoringProblem[] => {
    if (!Array.isArray(raw)) return []
    return raw.flatMap((entry): IMonitoringProblem[] => {
        if (!entry || typeof entry !== "object") return []
        const record = entry as Record<string, unknown>
        const text = (value: unknown) => (typeof value === "string" ? value : "")
        return [
            {
                severity:
                    SEVERITIES[text(record.severity).toLowerCase()] ??
                    EMonitoringProblemSeverity.ERROR,
                code: text(record.code),
                path: text(record.path),
                message: text(record.message),
                engine_code:
                    typeof record.engine_code === "string" ? record.engine_code : undefined,
            },
        ]
    })
}

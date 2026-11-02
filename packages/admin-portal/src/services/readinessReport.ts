// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {validFindings, type IReadinessFinding} from "./readinessFindings"

export const READINESS_CHECK_KINDS = [
    "configuration",
    "certified_voters",
    "official_zero",
    "publication",
    "tested_finished",
] as const

export interface IReadinessReport extends Record<string, unknown> {
    passed: boolean
    checked_at: string
    checks: Array<{
        kind: string
        passed: boolean
        message: string
        findings?: IReadinessFinding[]
    }>
    evidence: {
        official: Record<string, unknown> & {event_id: string}
        tested: Record<string, unknown> & {event_id: string}
    }
    log_entry_id: string | null
}

const isObject = (value: unknown): value is Record<string, unknown> =>
    value !== null && typeof value === "object" && !Array.isArray(value)

/** jsonb is untyped at the GraphQL boundary; never present an incomplete or unsigned pass. */
export function readinessReport(
    value: unknown,
    officialId: string,
    testedId: string
): IReadinessReport | null {
    if (
        !isObject(value) ||
        typeof value.passed !== "boolean" ||
        typeof value.checked_at !== "string" ||
        !value.checked_at ||
        !Array.isArray(value.checks) ||
        !isObject(value.evidence)
    )
        return null
    const {checks, evidence} = value
    if (
        !checks.every(
            (check: unknown) =>
                isObject(check) &&
                typeof check.kind === "string" &&
                typeof check.passed === "boolean" &&
                typeof check.message === "string" &&
                validFindings(check.findings) &&
                // A finding is a failed predicate: a passed check has none.
                !(check.passed && Array.isArray(check.findings) && check.findings.length)
        )
    )
        return null
    if (
        !READINESS_CHECK_KINDS.every(
            (kind) => checks.filter((check) => check.kind === kind).length === 1
        ) ||
        value.passed !== checks.every((check) => check.passed)
    )
        return null
    if (
        !isObject(evidence.official) ||
        evidence.official.event_id !== officialId ||
        !isObject(evidence.tested) ||
        evidence.tested.event_id !== testedId
    )
        return null
    if (
        value.passed
            ? typeof value.log_entry_id !== "string" || !value.log_entry_id.trim()
            : value.log_entry_id !== null
    )
        return null
    return value as IReadinessReport
}

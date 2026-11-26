// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {TFunction} from "i18next"
import {findingText, validFindings, type IReadinessFinding} from "./readinessFindings"

/** The server's ComparisonMode. Only "production" can record signed evidence. */
export const READINESS_COMPARISONS = [
    "production",
    "rehearsal_ert_stl",
    "rehearsal_official_ert",
    "rehearsal_official_stl",
] as const
export type ReadinessComparison = (typeof READINESS_COMPARISONS)[number]
export type RehearsalComparison = Exclude<ReadinessComparison, "production">

export const REHEARSAL_VERDICTS = ["consistent", "inconsistent", "not_verified"] as const
export type RehearsalVerdict = (typeof REHEARSAL_VERDICTS)[number]

export const REHEARSAL_CHECK_KINDS = [
    "configuration",
    "own_census",
    "census_subset",
    "lifecycle",
    "seal",
] as const

type Role = "official" | "tested" | "ert" | "stl"

/** The roles of the first (current event) and second (selected event) positions. */
export const comparisonRoles = (comparison: ReadinessComparison): [Role, Role] => {
    switch (comparison) {
        case "rehearsal_ert_stl":
            return ["ert", "stl"]
        case "rehearsal_official_ert":
            return ["official", "ert"]
        case "rehearsal_official_stl":
            return ["official", "stl"]
        default:
            return ["official", "tested"]
    }
}

export interface IRehearsalReport extends Record<string, unknown> {
    comparison: RehearsalComparison
    verdict: RehearsalVerdict
    checked_at: string
    checks: Array<{
        kind: (typeof REHEARSAL_CHECK_KINDS)[number]
        verdict: RehearsalVerdict
        message: string
        findings?: IReadinessFinding[]
    }>
    reference: {role: string; evidence: Record<string, unknown> & {event_id: string}}
    compared: {role: string; evidence: Record<string, unknown> & {event_id: string}}
}

const isObject = (value: unknown): value is Record<string, unknown> =>
    value !== null && typeof value === "object" && !Array.isArray(value)

const known = <T extends string>(list: readonly T[], value: unknown): value is T =>
    list.some((item) => item === value)

const aggregate = (verdicts: RehearsalVerdict[]): RehearsalVerdict =>
    verdicts.includes("inconsistent")
        ? "inconsistent"
        : verdicts.includes("not_verified")
          ? "not_verified"
          : "consistent"

/**
 * jsonb is untyped at the GraphQL boundary. A rehearsal report must name the
 * requested read-only comparison and can never carry a readiness pass or a
 * signed log entry; the official/STL subset can never be consistent.
 */
export function rehearsalReport(
    value: unknown,
    comparison: RehearsalComparison,
    referenceId: string,
    comparedId: string
): IRehearsalReport | null {
    if (
        !isObject(value) ||
        value.comparison !== comparison ||
        "passed" in value ||
        "log_entry_id" in value ||
        "evidence" in value ||
        !known(REHEARSAL_VERDICTS, value.verdict) ||
        typeof value.checked_at !== "string" ||
        !value.checked_at ||
        !Array.isArray(value.checks)
    )
        return null
    const checks: unknown[] = value.checks
    if (
        !checks.every(
            (check) =>
                isObject(check) &&
                known(REHEARSAL_CHECK_KINDS, check.kind) &&
                known(REHEARSAL_VERDICTS, check.verdict) &&
                typeof check.message === "string" &&
                validFindings(check.findings) &&
                // A consistent check has no findings; any other names why.
                (check.verdict === "consistent") ===
                    !(Array.isArray(check.findings) && check.findings.length)
        )
    )
        return null
    const typed = checks as IRehearsalReport["checks"]
    const subset = comparison === "rehearsal_official_stl"
    const required = REHEARSAL_CHECK_KINDS.filter((kind) => subset || kind !== "census_subset")
    if (
        typed.length !== required.length ||
        !required.every((kind) => typed.filter((check) => check.kind === kind).length === 1)
    )
        return null
    if (typed.some((check) => check.kind === "census_subset" && check.verdict === "consistent"))
        return null
    const verdict = aggregate(typed.map((check) => check.verdict))
    if (value.verdict !== verdict || (subset && verdict === "consistent")) return null
    const [left, right] = comparisonRoles(comparison)
    for (const [side, role, id] of [
        [value.reference, left, referenceId],
        [value.compared, right, comparedId],
    ] as const) {
        if (
            !isObject(side) ||
            side.role !== role ||
            !isObject(side.evidence) ||
            side.evidence.event_id !== id
        )
            return null
    }
    return value as IRehearsalReport
}

/** Codes whose production explanation names the event by role, never "tested". */
const ROLE_NEUTRAL_CODES = [
    "configuration_missing",
    "areas_mismatch",
    "voter_import_missing",
    "voter_import_in_progress",
    "voter_file_not_certified",
    "voter_receipt_not_signed",
    "registry_count_changed",
    "voter_area_not_configured",
    "ballots_present",
    "received_ballots_present",
    "tally_sessions_present",
    "status_missing",
    "voting_started",
    "no_elections",
    "publication_missing",
    "publication_mismatch",
    "publication_empty",
]

const REHEARSAL_CODES = ["revision_not_distinct", "subset_area_exceeds", "subset_not_verified"]

/** Translated where the wording fits a rehearsal; otherwise the server's words. */
export const rehearsalFindingText = (t: TFunction, finding: IReadinessFinding): string => {
    if (REHEARSAL_CODES.includes(finding.code))
        return String(
            t(`readiness.rehearsal.findings.${finding.code}`, {
                ...(finding.values ?? {}),
                area: finding.area?.trim() ? finding.area : String(t("readiness.noArea")),
                interpolation: {escapeValue: false},
            })
        )
    if (ROLE_NEUTRAL_CODES.includes(finding.code)) return findingText(t, finding)
    return finding.message || finding.code
}

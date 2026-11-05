// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {TFunction} from "i18next"

/** The server's FindingCode: why a readiness check failed. */
export const READINESS_FINDING_CODES = [
    "same_event",
    "configuration_missing",
    "configuration_incomplete",
    "external_id_mismatch",
    "content_mismatch",
    "approvers_mismatch",
    "areas_mismatch",
    "voter_import_missing",
    "voter_import_in_progress",
    "voter_file_not_certified",
    "voter_receipt_not_signed",
    "registry_count_changed",
    "voter_area_not_configured",
    "voter_file_mismatch",
    "voter_count_mismatch",
    "ballots_present",
    "received_ballots_present",
    "tally_sessions_present",
    "status_missing",
    "voting_started",
    "no_elections",
    "publication_missing",
    "publication_mismatch",
    "publication_empty",
    "channels_open",
    "seal_at_close_off",
    "no_expected_boxes",
    "boxes_unsealed",
] as const

export const READINESS_CERTIFICATION_REASONS = [
    "declared_sha256_missing",
    "declared_sha256_mismatch",
    "no_counts",
    "blank_area",
    "invalid_sha256",
] as const

const EVENT_ROLES = ["official", "tested", "both"] as const

export interface IReadinessFinding {
    code: string
    event: string
    area?: string
    election_id?: string
    values?: Record<string, string>
    message: string
}

const isObject = (value: unknown): value is Record<string, unknown> =>
    value !== null && typeof value === "object" && !Array.isArray(value)

const known = <T extends string>(list: readonly T[], value: unknown): value is T =>
    list.some((item) => item === value)

/** Findings are optional: reports from older servers have none. */
export const validFindings = (value: unknown): boolean =>
    value === undefined ||
    (Array.isArray(value) &&
        value.every(
            (finding) =>
                isObject(finding) &&
                typeof finding.code === "string" &&
                typeof finding.event === "string" &&
                typeof finding.message === "string"
        ))

/**
 * A translated explanation built from the server's observed values. A code
 * this portal does not know keeps the server's own words. React escapes the
 * result, so interpolated evidence is never read as markup.
 */
export const findingText = (t: TFunction, finding: IReadinessFinding): string => {
    if (!known(READINESS_FINDING_CODES, finding.code)) return finding.message || finding.code
    const values = finding.values ?? {}
    const event = known(EVENT_ROLES, finding.event)
        ? String(t(`readiness.eventRoles.${finding.event}`))
        : finding.event
    const scope = finding.election_id
        ? String(t("readiness.scope.election", {election: finding.election_id}))
        : String(t("readiness.scope.event", {event}))
    const key =
        finding.code === "voting_started" &&
        values.status === "NOT_STARTED" &&
        values.dates === "recorded"
            ? "readiness.findings.voting_dates_recorded"
            : `readiness.findings.${finding.code}`
    return String(
        t(key, {
            ...values,
            event,
            scope,
            election: finding.election_id ?? "",
            area: finding.area?.trim() ? finding.area : String(t("readiness.noArea")),
            field: values.field ? String(t(`logsScreen.readiness.fields.${values.field}`)) : "",
            reason: known(READINESS_CERTIFICATION_REASONS, values.reason)
                ? String(t(`readiness.certification.${values.reason}`))
                : (values.reason ?? ""),
            interpolation: {escapeValue: false},
        })
    )
}

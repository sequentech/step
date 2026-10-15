// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {TFunction} from "i18next"
import {logDetails, type ILogMessage} from "./logMessage"
import {STATEMENT_KIND_READINESS_VERIFIED, STATEMENT_KIND_VOTERS_IMPORTED} from "./logEntryDetails"

const CHECK_KINDS = [
    "configuration",
    "certified_voters",
    "official_zero",
    "publication",
    "tested_finished",
] as const
const FIELD_NAMES = [
    "report",
    "passed",
    "checked_at",
    "evidence",
    "official",
    "tested",
    "official_event_id",
    "tested_event_id",
    "event_id",
    "election_id",
    "configuration",
    "external_id",
    "revision",
    "content_sha256",
    "manifest_sha256",
    "approvers",
    "areas",
    "voters",
    "import_id",
    "document_id",
    "file_name",
    "sha256",
    "declared_sha256",
    "counts_by_area",
    "imported_at",
    "voter_log_delivered",
    "current_voter_counts",
    "voter_import_in_progress",
    "status",
    "elections",
    "ballot_count",
    "received_ballot_count",
    "tally_session_count",
    "publication",
    "publication_id",
    "matches_manifest",
    "design_count",
    "seal_at_close",
    "expected_boxes",
    "sealed_boxes",
    "all_channels_closed",
    "locked_down",
    "log_entry_id",
]
const FIELD_ALIASES: Record<string, string> = {counts_by_post: "counts_by_area", posts: "areas"}
const isObject = (value: unknown): value is Record<string, unknown> =>
    !!value && typeof value === "object" && !Array.isArray(value)
const aliasFor = (name: string): string | undefined =>
    Object.prototype.hasOwnProperty.call(FIELD_ALIASES, name) ? FIELD_ALIASES[name] : undefined
const canonicalField = (name: string): string => aliasFor(name) ?? name
const fieldLabel = (t: TFunction, name: string): string => {
    const canonical = canonicalField(name)
    return FIELD_NAMES.includes(canonical)
        ? String(t(`logsScreen.readiness.fields.${canonical}`))
        : name
}
const isReadiness = (kind?: string | null): boolean =>
    kind === STATEMENT_KIND_VOTERS_IMPORTED || kind === STATEMENT_KIND_READINESS_VERIFIED

/** Translate the view only; historical signed field names and messages stay intact. */
export const readinessDescription = (t: TFunction, kind?: string | null): string | null =>
    isReadiness(kind) ? String(t(`logsScreen.readiness.description.${kind}`)) : null

/** CheckKind is the stable language-independent contract, including in old reports. */
export const readinessCheckLines = (t: TFunction, message: ILogMessage | null): string[] | null => {
    const details = logDetails(message)
    const report = isObject(details?.report) ? details.report : details
    if (!Array.isArray(report?.checks)) return null
    return report.checks.filter(isObject).map((check) => {
        const kind = typeof check.kind === "string" ? check.kind : String(check.name ?? "")
        const description = CHECK_KINDS.some((known) => known === kind)
            ? String(t(`logsScreen.readiness.checks.${kind}`))
            : kind
        const outcome = String(
            t(check.passed === true ? "logsScreen.readiness.passed" : "logsScreen.readiness.failed")
        )
        return String(t("logsScreen.readiness.checkResult", {outcome, description}))
    })
}

const shownValue = (t: TFunction, value: unknown, field?: string): string => {
    if (value === null || value === undefined) return String(t("logsScreen.entry.values.none"))
    if (typeof value === "boolean")
        return String(t(value ? "logsScreen.entry.values.yes" : "logsScreen.entry.values.no"))
    if (Array.isArray(value)) return value.map((item) => shownValue(t, item)).join(", ")
    if (!isObject(value)) return String(value)
    if (field === "counts_by_area" || field === "current_voter_counts") {
        return Object.entries(value)
            .map(([name, count]) => `${name}: ${shownValue(t, count)}`)
            .join("; ")
    }
    return (
        Object.entries(value)
            // Checks have their own translated explanation; never surface old backend prose.
            .filter(
                ([name]) => name !== "checks" && !(aliasFor(name) && canonicalField(name) in value)
            )
            .map(
                ([name, inner]) =>
                    `${fieldLabel(t, name)}: ${shownValue(t, inner, canonicalField(name))}`
            )
            .join("; ")
    )
}

/** Known receipt/report labels, with dynamic area identifiers preserved as data. */
export const readinessDetailRows = (
    t: TFunction,
    kind: string | null | undefined,
    message: ILogMessage | null
): Array<{name: string; value: string}> | null => {
    if (!isReadiness(kind)) return null
    const details = logDetails(message)
    if (!details) return null
    return Object.entries(details)
        .filter(
            ([name]) =>
                name !== "actor" &&
                name !== "checks" &&
                !(aliasFor(name) && canonicalField(name) in details)
        )
        .map(([name, value]) => ({
            name: fieldLabel(t, name),
            value: shownValue(t, value, canonicalField(name)),
        }))
}

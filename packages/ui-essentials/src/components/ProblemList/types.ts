// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * What `sequent-core` says is wrong with a file somebody is importing.
 *
 * These mirror `election_config::problem` in `sequent-core`, which is where they
 * are defined and serialized: the Election Architect gets them from the WASM
 * build, the Admin Portal from an import action or a failed task's annotations.
 * Repeated here so the components take data rather than a core, and render in a
 * test with no WASM present.
 */

/** Whether a problem stops an import or merely deserves saying out loud. */
export type ProblemSeverity = "error" | "warning"

/**
 * A stable category for a kind of problem.
 *
 * Match on this rather than on `message`: the wording is free to change, the code
 * is not. The codes the core can produce, plus `string` so a front end built
 * against an older package still compiles when a code is added.
 */
export type ProblemCode =
    | "missing_field"
    | "invalid_value"
    | "dangling_reference"
    | "duplicate_id"
    | "area_cycle"
    | "contest_arithmetic"
    | "tally_mismatch"
    | "ballot_coverage"
    | "permission_label"
    | "missing_schedule"
    | "conflicting_columns"
    | "unreadable"
    | "incompatible_version"
    | "integrity_mismatch"
    | (string & {})

export interface Problem {
    severity: ProblemSeverity
    code: ProblemCode
    /**
     * Where: a dotted path into the bundle (`contests[2].max_votes`), or a sheet
     * and row for a source document (`row 4 column 'vote-weight'`).
     */
    path: string
    /** The core's own English, shown when nothing better is known. */
    message: string
    /**
     * A stable name for this sentence, and the key it is translated under:
     * `problems.messages.<id>.text`.
     *
     * `code` cannot do this job: a code is a *category*, and four different
     * complaints about one contest share a code and a path. Absent means "no
     * translation exists for this one, show `message`".
     */
    id?: string
    /**
     * The specifics `message` names — an address, a row, a pair of numbers —
     * which a translated sentence interpolates.
     *
     * **Absent rather than `{}`** when a problem has none: the core skips the
     * field when empty.
     */
    details?: Record<string, string>
    /** The entity's `external_id`, where it has one. */
    external_id?: string
    /** Where in a source spreadsheet, structurally, when the problem is about one. */
    at?: {sheet: string; row?: number; column?: string}
}

export interface ProblemReport {
    problems: Problem[]
}

export const errorsOf = (report: ProblemReport): Problem[] =>
    report.problems.filter((problem) => problem.severity === "error")

export const warningsOf = (report: ProblemReport): Problem[] =>
    report.problems.filter((problem) => problem.severity === "warning")

/** Whether the file should be refused. */
export const hasErrors = (report: ProblemReport): boolean =>
    report.problems.some((problem) => problem.severity === "error")

/**
 * Problems out of whatever a server sent, or nothing.
 *
 * An import action's `problems`, and a task's `annotations.problems`, are JSON
 * columns: typed `any` by the GraphQL codegen, absent from an older backend, and
 * never to be trusted to be the shape they should. What is not an array of
 * objects with a severity and a message is dropped rather than rendered as
 * `undefined`, and nothing at all comes back as `undefined` so a caller falls
 * back to its plain error text.
 */
export const readProblems = (value: unknown): Problem[] | undefined => {
    if (!Array.isArray(value)) {
        return undefined
    }
    const problems = value.filter(
        (each): each is Problem =>
            typeof each === "object" &&
            each !== null &&
            (each.severity === "error" || each.severity === "warning") &&
            typeof each.message === "string"
    )
    return problems.length > 0 ? problems : undefined
}

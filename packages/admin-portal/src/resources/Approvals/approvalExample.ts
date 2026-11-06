// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The example the approval matrix screen decides: the sample, or the inputs
// recorded when an enrollment was decided, and how its current simulation
// relates to that recorded decision.
import {IApplicationsStatus} from "@/types/applications"
import {EFieldMatch, EIdentityMethod, EMatrixInvariant, ITestEnrollment} from "./approvalMatrix"
import {IApplication, applicantData, jointNames} from "./approvalReview"

const MIDDLE_NAME = "middleName"

/** Where the approval matrix screen was opened from. */
export enum EMatrixEntry {
    TOOLBAR = "TOOLBAR",
    ENROLLMENT = "ENROLLMENT",
}

/** What the example says about one compared detail. */
export enum EFieldState {
    MATCHES = "MATCHES",
    DIFFERS = "DIFFERS",
    /** The ID gives first and middle name together, so the middle name has no result of its own. */
    WITH_FIRST_NAME = "WITH_FIRST_NAME",
    NOT_RECORDED = "NOT_RECORDED",
}

/** How the current simulation relates to the decision recorded for the enrollment. */
export enum EHistoryComparison {
    SAME = "SAME",
    DIFFERENT = "DIFFERENT",
    EDITED = "EDITED",
    NOT_RECORDED = "NOT_RECORDED",
    SEVERAL_VOTERS = "SEVERAL_VOTERS",
    UNKNOWN_VOTERS = "UNKNOWN_VOTERS",
}

/** The decision stored with an application, read strictly so it can be replayed. */
export interface IRecordedDecision {
    matrixVersion: number
    /** The rule's position starting at 1; empty for the last rule. */
    rule: number | null
    decision: IApplicationsStatus | null
    invariant: EMatrixInvariant | null
    /** How many registry voters the rules approved; empty when not recorded. */
    approvedVoters: number | null
    /** How many registry voters were compared; empty when not recorded. */
    candidates: number | null
    /** Whether the enrollment's ID gives first and middle name together. */
    jointNames: boolean
    /** The inputs the rules saw; empty when they can't be read as recorded. */
    enrollment: ITestEnrollment | null
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

/** A recorded count or position: a whole number from 0, or empty. */
const count = (value: unknown): number | null =>
    typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? value : null

const isMember = <T extends string>(values: Record<string, T>, value: unknown): value is T =>
    Object.values(values).some((candidate) => candidate === value)

function recordedEnrollment(value: unknown): ITestEnrollment | null {
    if (!isRecord(value)) {
        return null
    }
    const {identity, voter_found, already_enrolled, valid_id} = value
    const fields = value.fields === undefined ? {} : value.fields
    if (
        typeof voter_found !== "boolean" ||
        typeof already_enrolled !== "boolean" ||
        !(identity === null || isMember(EIdentityMethod, identity)) ||
        !(valid_id === null || typeof valid_id === "string") ||
        !isRecord(fields)
    ) {
        return null
    }
    const results: Record<string, EFieldMatch> = {}
    for (const [field, result] of Object.entries(fields)) {
        if (!isMember(EFieldMatch, result)) {
            return null
        }
        results[field] = result
    }
    return {identity, voter_found, already_enrolled, valid_id, fields: results}
}

/**
 * Recorded inputs describe a registry voter exactly when voters were compared;
 * a record that says otherwise isn't replayed.
 */
const consistent = (
    enrollment: ITestEnrollment | null,
    candidates: number | null
): ITestEnrollment | null =>
    enrollment && candidates !== null && enrollment.voter_found !== candidates > 0
        ? null
        : enrollment

/** The decision recorded for an application; empty when it has none. */
export function recordedDecision(application: IApplication): IRecordedDecision | null {
    const record = isRecord(application.annotations) ? application.annotations.decision : null
    if (!isRecord(record) || typeof record.matrix_version !== "number") {
        return null
    }
    const {rule, decision, invariant} = record
    const version = count(record.matrix_version)
    const readable =
        version !== null &&
        version > 0 &&
        isMember(IApplicationsStatus, decision) &&
        (rule === null || (count(rule) ?? 0) > 0) &&
        (invariant === null || invariant === undefined || isMember(EMatrixInvariant, invariant))
    return {
        matrixVersion: record.matrix_version,
        rule: readable ? count(rule) : null,
        decision: readable ? decision : null,
        invariant: readable && isMember(EMatrixInvariant, invariant) ? invariant : null,
        approvedVoters: count(record.accepted_candidates),
        candidates: count(record.candidates),
        jointNames: jointNames(applicantData(application)),
        enrollment: readable
            ? consistent(recordedEnrollment(record.inputs), count(record.candidates))
            : null,
    }
}

/** The toolbar's example: a verified voter in the registry whose compared details are all the same. */
export const sampleEnrollment = (): ITestEnrollment => ({
    identity: EIdentityMethod.VERIFIED,
    voter_found: true,
    already_enrolled: false,
    valid_id: null,
    fields: {},
})

/** What the example says about a compared detail. The sample's details are the same unless chosen. */
export function fieldState(
    enrollment: ITestEnrollment,
    field: string,
    entry: EMatrixEntry,
    joint: boolean
): EFieldState {
    const result = enrollment.fields[field]
    if (result) {
        return result === EFieldMatch.MATCHES ? EFieldState.MATCHES : EFieldState.DIFFERS
    }
    if (entry === EMatrixEntry.TOOLBAR) {
        return EFieldState.MATCHES
    }
    return joint && field === MIDDLE_NAME ? EFieldState.WITH_FIRST_NAME : EFieldState.NOT_RECORDED
}

/**
 * The enrollment sent to be decided: only the compared details. A detail
 * without a result is left out; the rules treat it as differing in field
 * conditions and don't count it among the details that differ.
 */
export function enrollmentFor(
    enrollment: ITestEnrollment,
    comparedFields: string[],
    entry: EMatrixEntry,
    joint: boolean
): ITestEnrollment {
    const fields: Record<string, EFieldMatch> = {}
    if (enrollment.voter_found) {
        comparedFields.forEach((field) => {
            const state = fieldState(enrollment, field, entry, joint)
            if (state === EFieldState.MATCHES || state === EFieldState.DIFFERS) {
                fields[field] =
                    state === EFieldState.MATCHES ? EFieldMatch.MATCHES : EFieldMatch.DIFFERS
            }
        })
    }
    return {
        ...enrollment,
        already_enrolled: enrollment.voter_found && enrollment.already_enrolled,
        fields,
    }
}

const sortedFields = (fields: Record<string, EFieldMatch>) =>
    JSON.stringify(Object.entries(fields).sort(([a], [b]) => a.localeCompare(b)))

export const sameEnrollment = (a: ITestEnrollment, b: ITestEnrollment): boolean =>
    a.identity === b.identity &&
    a.voter_found === b.voter_found &&
    a.already_enrolled === b.already_enrolled &&
    a.valid_id === b.valid_id &&
    sortedFields(a.fields) === sortedFields(b.fields)

/** Whether the toolbar's example was changed from the sample on what the rules see. */
export const customizedSample = (enrollment: ITestEnrollment, comparedFields: string[]): boolean =>
    !sameEnrollment(
        enrollmentFor(enrollment, comparedFields, EMatrixEntry.TOOLBAR, false),
        enrollmentFor(sampleEnrollment(), comparedFields, EMatrixEntry.TOOLBAR, false)
    )

/**
 * How the decision of the applied example with the rules on screen relates
 * to the recorded one; empty when there is nothing to say. The decisions are
 * compared only when the applied example replays the record exactly.
 */
export function historyComparison(
    recorded: IRecordedDecision,
    applied: ITestEnrollment,
    comparedFields: string[],
    decision: IApplicationsStatus | null
): EHistoryComparison | null {
    if (!recorded.enrollment || !recorded.decision || !decision) {
        return null
    }
    const project = (enrollment: ITestEnrollment) =>
        enrollmentFor(enrollment, comparedFields, EMatrixEntry.ENROLLMENT, recorded.jointNames)
    if (!sameEnrollment(project(applied), project(recorded.enrollment))) {
        return EHistoryComparison.EDITED
    }
    if (recorded.candidates === null) {
        return EHistoryComparison.UNKNOWN_VOTERS
    }
    if (recorded.candidates > 1) {
        return EHistoryComparison.SEVERAL_VOTERS
    }
    if (
        applied.voter_found &&
        comparedFields.some(
            (field) =>
                fieldState(applied, field, EMatrixEntry.ENROLLMENT, recorded.jointNames) ===
                EFieldState.NOT_RECORDED
        )
    ) {
        return EHistoryComparison.NOT_RECORDED
    }
    return decision === recorded.decision ? EHistoryComparison.SAME : EHistoryComparison.DIFFERENT
}

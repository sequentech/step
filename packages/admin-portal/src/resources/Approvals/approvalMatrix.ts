// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IApplicationsStatus} from "@/types/applications"
import {convertToCamelCase} from "./UtilsApprovals"

export enum EIdentityMethod {
    VERIFIED = "VERIFIED",
    MANUAL_ENTRY = "MANUAL_ENTRY",
}

export enum EFieldMatch {
    MATCHES = "MATCHES",
    DIFFERS = "DIFFERS",
}

export enum EDifferingFields {
    NONE = "none",
    EXACTLY_1 = "exactly_1",
    AT_MOST_1 = "at_most_1",
    EXACTLY_2 = "exactly_2",
    AT_MOST_2 = "at_most_2",
    AT_LEAST_3 = "at_least_3",
}

export enum EMatrixReason {
    NO_VOTER = "NO_VOTER",
    ALREADY_APPROVED = "ALREADY_APPROVED",
    INSUFFICIENT_INFORMATION = "INSUFFICIENT_INFORMATION",
    IDENTITY_NOT_VERIFIED = "IDENTITY_NOT_VERIFIED",
    OTHER = "OTHER",
}

export enum EMatrixSource {
    BUILT_IN = "BUILT_IN",
    SAVED = "SAVED",
}

export enum EMatrixError {
    NO_COMPARED_FIELDS = "NO_COMPARED_FIELDS",
    DUPLICATE_COMPARED_FIELD = "DUPLICATE_COMPARED_FIELD",
    UNKNOWN_FIELD = "UNKNOWN_FIELD",
    ACCEPTS_MANUAL_ENTRY = "ACCEPTS_MANUAL_ENTRY",
    ACCEPTS_ALREADY_ENROLLED = "ACCEPTS_ALREADY_ENROLLED",
    ACCEPTS_WITHOUT_VOTER = "ACCEPTS_WITHOUT_VOTER",
    OTHERWISE_ACCEPTS = "OTHERWISE_ACCEPTS",
    MISSING_REASON = "MISSING_REASON",
    UNEXPECTED_REASON = "UNEXPECTED_REASON",
}

export interface IRuleConditions {
    identity?: EIdentityMethod
    voter_found?: boolean
    already_enrolled?: boolean
    valid_id?: string
    differing?: EDifferingFields
    fields?: Record<string, EFieldMatch>
}

export interface IRuleOutcome {
    decision: IApplicationsStatus
    reason?: EMatrixReason
}

export interface IApprovalRule {
    when: IRuleConditions
    then: IRuleOutcome
}

export interface IApprovalMatrix {
    compared_fields: string[]
    rules: IApprovalRule[]
    otherwise: IRuleOutcome
}

/** An enrollment as the test panel describes it. */
export interface ITestEnrollment {
    identity: EIdentityMethod | null
    voter_found: boolean
    already_enrolled: boolean
    valid_id: string | null
    fields: Record<string, EFieldMatch>
}

/** The matrix version, rule and inputs stored with an application. */
export interface IDecisionRecord {
    matrix_version: number
    rule: number | null
    conditions: IRuleConditions | null
}

export type Translate = (key: string, options?: Record<string, unknown>) => string
export type FieldLabel = (field: string) => string

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const isMember = <T extends string>(values: Record<string, T>, value: unknown): value is T =>
    Object.values(values).some((candidate) => candidate === value)

function readConditions(value: unknown): IRuleConditions {
    if (!isRecord(value)) {
        return {}
    }
    const conditions: IRuleConditions = {}
    if (isMember(EIdentityMethod, value.identity)) {
        conditions.identity = value.identity
    }
    if (typeof value.voter_found === "boolean") {
        conditions.voter_found = value.voter_found
    }
    if (typeof value.already_enrolled === "boolean") {
        conditions.already_enrolled = value.already_enrolled
    }
    if (typeof value.valid_id === "string" && value.valid_id) {
        conditions.valid_id = value.valid_id
    }
    if (isMember(EDifferingFields, value.differing)) {
        conditions.differing = value.differing
    }
    if (isRecord(value.fields)) {
        const fields: Record<string, EFieldMatch> = {}
        Object.entries(value.fields).forEach(([field, result]) => {
            if (isMember(EFieldMatch, result)) {
                fields[field] = result
            }
        })
        if (Object.keys(fields).length > 0) {
            conditions.fields = fields
        }
    }
    return conditions
}

function readOutcome(value: unknown): IRuleOutcome | null {
    if (!isRecord(value) || !isMember(IApplicationsStatus, value.decision)) {
        return null
    }
    return isMember(EMatrixReason, value.reason)
        ? {decision: value.decision, reason: value.reason}
        : {decision: value.decision}
}

/** Reads a matrix as the backend returns it; empty when it isn't one. */
export function readMatrix(value: unknown): IApprovalMatrix | null {
    if (!isRecord(value) || !Array.isArray(value.compared_fields) || !Array.isArray(value.rules)) {
        return null
    }
    const otherwise = readOutcome(value.otherwise)
    if (!otherwise) {
        return null
    }
    const rules: IApprovalRule[] = []
    for (const rule of value.rules) {
        const then = isRecord(rule) ? readOutcome(rule.then) : null
        if (!isRecord(rule) || !then) {
            return null
        }
        rules.push({when: readConditions(rule.when), then})
    }
    return {
        compared_fields: value.compared_fields.filter(
            (field): field is string => typeof field === "string"
        ),
        rules,
        otherwise,
    }
}

/** Reads the decision stored in an application's annotations. */
export function readDecision(value: unknown): IDecisionRecord | null {
    if (!isRecord(value) || typeof value.matrix_version !== "number") {
        return null
    }
    return {
        matrix_version: value.matrix_version,
        rule: typeof value.rule === "number" ? value.rule : null,
        conditions: isRecord(value.conditions) ? readConditions(value.conditions) : null,
    }
}

/** Drops the reason of an accepting outcome, which has none. */
function cleanOutcome(outcome: IRuleOutcome): IRuleOutcome {
    return outcome.decision === IApplicationsStatus.ACCEPTED || !outcome.reason
        ? {decision: outcome.decision}
        : {decision: outcome.decision, reason: outcome.reason}
}

/** Keeps only the conditions that are set, on fields the matrix compares. */
export function cleanConditions(when: IRuleConditions, comparedFields: string[]): IRuleConditions {
    const conditions = readConditions(when)
    if (conditions.fields) {
        const fields = Object.fromEntries(
            Object.entries(conditions.fields).filter(([field]) => comparedFields.includes(field))
        )
        if (Object.keys(fields).length > 0) {
            conditions.fields = fields
        } else {
            delete conditions.fields
        }
    }
    return conditions
}

/** The matrix as the backend accepts it: no empty conditions and no unknown keys. */
export function cleanMatrix(matrix: IApprovalMatrix): IApprovalMatrix {
    const comparedFields = matrix.compared_fields
        .map((field) => field.trim())
        .filter((field, index, fields) => field && fields.indexOf(field) === index)
    return {
        compared_fields: comparedFields,
        rules: matrix.rules.map((rule) => ({
            when: cleanConditions(rule.when, comparedFields),
            then: cleanOutcome(rule.then),
        })),
        otherwise: cleanOutcome(matrix.otherwise),
    }
}

export const sameMatrix = (a: IApprovalMatrix, b: IApprovalMatrix): boolean =>
    JSON.stringify(sortedKeys(cleanMatrix(a))) === JSON.stringify(sortedKeys(cleanMatrix(b)))

function sortedKeys(value: unknown): unknown {
    if (Array.isArray(value)) {
        return value.map(sortedKeys)
    }
    if (isRecord(value)) {
        return Object.fromEntries(
            Object.keys(value)
                .sort()
                .map((key) => [key, sortedKeys(value[key])])
        )
    }
    return value
}

/** Why a rule can't be applied in the editor. `isOtherwise` is the last rule. */
export function validateRule(rule: IApprovalRule, isOtherwise = false): EMatrixError[] {
    const errors: EMatrixError[] = []
    if (rule.then.decision === IApplicationsStatus.ACCEPTED) {
        if (isOtherwise) {
            return [EMatrixError.OTHERWISE_ACCEPTS]
        }
        if (rule.when.identity === EIdentityMethod.MANUAL_ENTRY) {
            errors.push(EMatrixError.ACCEPTS_MANUAL_ENTRY)
        }
        if (rule.when.already_enrolled === true) {
            errors.push(EMatrixError.ACCEPTS_ALREADY_ENROLLED)
        }
        if (rule.when.voter_found === false) {
            errors.push(EMatrixError.ACCEPTS_WITHOUT_VOTER)
        }
    } else if (!rule.then.reason) {
        errors.push(EMatrixError.MISSING_REASON)
    }
    return errors
}

/** Every rule's errors, with the rule's position starting at 1 (empty for the last rule). */
export function validateMatrix(
    matrix: IApprovalMatrix
): Array<{code: EMatrixError; rule: number | null}> {
    const clean = cleanMatrix(matrix)
    const errors: Array<{code: EMatrixError; rule: number | null}> = []
    if (clean.compared_fields.length === 0) {
        errors.push({code: EMatrixError.NO_COMPARED_FIELDS, rule: null})
    }
    clean.rules.forEach((rule, index) => {
        validateRule(rule).forEach((code) => errors.push({code, rule: index + 1}))
    })
    validateRule({when: {}, then: clean.otherwise}, true).forEach((code) =>
        errors.push({code, rule: null})
    )
    return errors
}

export const addRule = (matrix: IApprovalMatrix, rule: IApprovalRule): IApprovalMatrix => ({
    ...matrix,
    rules: [...matrix.rules, rule],
})

export const replaceRule = (
    matrix: IApprovalMatrix,
    index: number,
    rule: IApprovalRule
): IApprovalMatrix => ({
    ...matrix,
    rules: matrix.rules.map((current, position) => (position === index ? rule : current)),
})

export const deleteRule = (matrix: IApprovalMatrix, index: number): IApprovalMatrix => ({
    ...matrix,
    rules: matrix.rules.filter((_, position) => position !== index),
})

/** Moves a rule one position up (-1) or down (1); unchanged at either end. */
export function moveRule(matrix: IApprovalMatrix, index: number, offset: -1 | 1): IApprovalMatrix {
    const target = index + offset
    if (index < 0 || index >= matrix.rules.length || target < 0 || target >= matrix.rules.length) {
        return matrix
    }
    const rules = [...matrix.rules]
    ;[rules[index], rules[target]] = [rules[target], rules[index]]
    return {...matrix, rules}
}

/** Changes the compared fields, dropping the conditions on fields no longer compared. */
export const withComparedFields = (matrix: IApprovalMatrix, fields: string[]): IApprovalMatrix =>
    cleanMatrix({...matrix, compared_fields: fields})

/** "dateOfBirth" as "Date Of Birth", for fields the user profile doesn't name. */
export const humanizeField = (field: string): string =>
    field
        .replace(/[._-]+/g, " ")
        .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
        .trim()
        .replace(/\b\w/g, (letter) => letter.toUpperCase())

const KEY = "approvalsScreen.matrix"

/** One line per condition, in the order the dialog shows them. */
export function conditionLabels(
    when: IRuleConditions,
    t: Translate,
    fieldLabel: FieldLabel = humanizeField
): string[] {
    const labels: string[] = []
    if (when.identity) {
        labels.push(t(`${KEY}.conditions.identity.${when.identity}`))
    }
    if (when.voter_found !== undefined) {
        labels.push(t(`${KEY}.conditions.voterFound.${when.voter_found}`))
    }
    if (when.already_enrolled !== undefined) {
        labels.push(t(`${KEY}.conditions.alreadyEnrolled.${when.already_enrolled}`))
    }
    if (when.valid_id) {
        labels.push(t(`${KEY}.conditions.validId`, {id: t(when.valid_id)}))
    }
    if (when.differing) {
        labels.push(t(`${KEY}.conditions.differing.${when.differing}`))
    }
    Object.entries(when.fields ?? {}).forEach(([field, result]) => {
        labels.push(t(`${KEY}.conditions.field.${result}`, {field: fieldLabel(field)}))
    })
    return labels.length > 0 ? labels : [t(`${KEY}.conditions.any`)]
}

/** "Approval matrix version 1, rule 5: Exactly 1 field differs, Embassy matches". */
export function decidedByText(
    decision: IDecisionRecord,
    t: Translate,
    fieldLabel: FieldLabel = humanizeField
): string {
    if (decision.rule === null) {
        return t("approvalsScreen.decision.otherwise", {version: decision.matrix_version})
    }
    return t("approvalsScreen.decision.text", {
        version: decision.matrix_version,
        rule: decision.rule,
        conditions: conditionLabels(decision.conditions ?? {}, t, fieldLabel).join(", "),
    })
}

/** The enrollment the test panel starts from: a verified voter whose fields all match. */
export const defaultEnrollment = (comparedFields: string[]): ITestEnrollment => ({
    identity: EIdentityMethod.VERIFIED,
    voter_found: true,
    already_enrolled: false,
    valid_id: null,
    fields: Object.fromEntries(comparedFields.map((field) => [field, EFieldMatch.MATCHES])),
})

/** The enrollment restricted to the fields the matrix compares now. */
export const enrollmentFor = (
    enrollment: ITestEnrollment,
    comparedFields: string[]
): ITestEnrollment => ({
    ...enrollment,
    fields: Object.fromEntries(
        comparedFields.map((field) => [field, enrollment.fields[field] ?? EFieldMatch.MATCHES])
    ),
})

interface IProfileAttribute {
    name?: string | null
    display_name?: string | null
}

/**
 * Names compared fields as the event's user profile does. `attributeLabel`
 * turns a profile display name into the text to translate.
 */
export const profileFieldLabel =
    (
        attributes: IProfileAttribute[],
        t: Translate,
        attributeLabel: (displayName: string) => string
    ): FieldLabel =>
    (field) => {
        const attribute = attributes.find(
            ({name}) => name && convertToCamelCase(name) === convertToCamelCase(field)
        )
        return attribute?.name
            ? t(attributeLabel(attribute.display_name ?? attribute.name))
            : humanizeField(field)
    }

/**
 * The translation key of an application's rejection reason. The matrix
 * stores a reason by its name; an officer's manual rejection by its slug.
 */
export const rejectionReasonKey = (reason: string | null | undefined): string =>
    Object.values<string>(EMatrixReason).includes(reason ?? "")
        ? `approvalsScreen.matrix.reasons.${reason}`
        : `approvalsScreen.reject.reasons.${reason ?? "undefined"}`

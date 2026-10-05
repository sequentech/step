// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the Approvals queue and the review of one enrollment say about an
// application: who applied, what happened to it and how it compares with a
// voter of the registry.
import {IApplicationsStatus} from "@/types/applications"
import {EFieldMatch, EIdentityMethod, FieldLabel, Translate, readConditions} from "./approvalMatrix"
import type {IRuleConditions} from "./approvalMatrix"

const KEY = "approvalsScreen"

/** Identity documents whose first and middle name are compared together. */
const JOINT_NAME_DOCUMENTS = ["seamanBook", "driversLicense"]
const ID_CARD_TYPE_FIELD = "sequent.read-only.id-card-type"
const FIRST_NAME = "firstName"
const MIDDLE_NAME = "middleName"

/** The voter attributes the registry lookup filters by name instead of as an attribute. */
const VOTER_COLUMNS: Record<string, "first_name" | "last_name" | "email" | "username"> = {
    firstName: "first_name",
    lastName: "last_name",
    email: "email",
    username: "username",
}

export interface IApplication {
    status?: string | null
    created_at?: string | null
    updated_at?: string | null
    applicant_data?: unknown
    annotations?: unknown
}

export interface IRegistryVoter {
    id?: string | null
    username?: string | null
    first_name?: string | null
    last_name?: string | null
    email?: string | null
    attributes?: Record<string, string[] | string | null | undefined> | null
}

/** What the rules saw of the enrollment, as stored with the application. */
export interface IDecisionDetails {
    matrixVersion: number
    /** The rule's position starting at 1; empty for the last rule. */
    rule: number | null
    conditions: IRuleConditions
    identity: EIdentityMethod | null
    voterFound: boolean
    alreadyEnrolled: boolean
    /** How many registry voters the rules would have approved; more than one needs a person. */
    approvedVoters: number
    fields: Record<string, EFieldMatch>
}

export interface IComparedDetail {
    /** The compared field, or the fields compared together. */
    fields: string[]
    enrollment: string
    registry: string
    same: boolean
}

export interface ICandidate {
    voter: IRegistryVoter
    details: IComparedDetail[]
    matching: number
    alreadyEnrolled: boolean
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const text = (value: unknown): string =>
    typeof value === "string" ? value.trim() : typeof value === "number" ? String(value) : ""

/** The applicant's answers by attribute name. */
export function applicantData(application: IApplication): Record<string, string> {
    const data = application.applicant_data
    if (!isRecord(data)) {
        return {}
    }
    return Object.fromEntries(Object.entries(data).map(([key, value]) => [key, text(value)]))
}

const annotations = (application: IApplication): Record<string, unknown> =>
    isRecord(application.annotations) ? application.annotations : {}

/** "Juan Carlos Dela Cruz", from the names on the enrollment. */
export function applicantName(application: IApplication): string {
    const data = applicantData(application)
    return [data.firstName, data.middleName, data.lastName].filter((part) => part).join(" ")
}

/** "JC" for "Juan Carlos Dela Cruz": the first letters of the first and last word. */
export function initials(name: string): string {
    const words = name.split(/\s+/).filter((word) => word)
    if (words.length === 0) {
        return ""
    }
    const letters = words.length === 1 ? [words[0]] : [words[0], words[words.length - 1]]
    return letters.map((word) => word.charAt(0).toUpperCase()).join("")
}

/** A comma separated annotation as a list. */
export function listAnnotation(application: IApplication, name: string): string[] {
    const value = annotations(application)[name]
    return typeof value === "string"
        ? value
              .split(",")
              .map((item) => item.trim())
              .filter((item) => item)
        : []
}

/** The decision record of an application; empty when it has none. */
export function decisionDetails(application: IApplication): IDecisionDetails | null {
    const record = annotations(application).decision
    if (!isRecord(record) || typeof record.matrix_version !== "number") {
        return null
    }
    const inputs = isRecord(record.inputs) ? record.inputs : {}
    const fields: Record<string, EFieldMatch> = {}
    Object.entries(isRecord(inputs.fields) ? inputs.fields : {}).forEach(([field, result]) => {
        if (result === EFieldMatch.MATCHES || result === EFieldMatch.DIFFERS) {
            fields[field] = result
        }
    })
    return {
        matrixVersion: record.matrix_version,
        rule: typeof record.rule === "number" ? record.rule : null,
        conditions: readConditions(record.conditions),
        identity:
            inputs.identity === EIdentityMethod.VERIFIED ||
            inputs.identity === EIdentityMethod.MANUAL_ENTRY
                ? inputs.identity
                : null,
        voterFound: inputs.voter_found === true,
        alreadyEnrolled: inputs.already_enrolled === true,
        approvedVoters:
            typeof record.accepted_candidates === "number" ? record.accepted_candidates : 0,
        fields,
    }
}

/** The compared fields of the decision that differ from the registry. */
export const differingFields = (decision: IDecisionDetails): string[] =>
    Object.entries(decision.fields)
        .filter(([, result]) => result === EFieldMatch.DIFFERS)
        .map(([field]) => field)

/** "first name, last name and embassy". */
export function joinList(items: string[], t: Translate): string {
    if (items.length < 2) {
        return items.join("")
    }
    return t(`${KEY}.summary.join`, {
        head: items.slice(0, -1).join(", "),
        last: items[items.length - 1],
    })
}

const capitalize = (value: string): string => value.charAt(0).toUpperCase() + value.slice(1)

/** The officer who decided the application by hand; empty when the rules did. */
export function decidedBy(application: IApplication): string {
    return text(annotations(application).verified_by)
}

/**
 * The two lines of the queue's "What happened" column: what the rules found,
 * and how the identity was established or who decided.
 */
export function enrollmentSummary(
    application: IApplication,
    t: Translate,
    fieldLabel: FieldLabel
): {headline: string; detail: string} {
    const decision = decisionDetails(application)
    const officer = decidedBy(application)
    const manual = decision?.identity === EIdentityMethod.MANUAL_ENTRY
    const differing = decision && decision.voterFound ? differingFields(decision) : []
    const found =
        differing.length > 0
            ? capitalize(
                  t(`${KEY}.summary.differs`, {
                      count: differing.length,
                      fields: joinList(
                          differing.map((field) => fieldLabel(field).toLowerCase()),
                          t
                      ),
                  })
              )
            : decision && !decision.voterFound
              ? t(`${KEY}.summary.noVoter`)
              : decision
                ? t(`${KEY}.summary.allMatch`)
                : ""
    const identity = manual
        ? t(`${KEY}.summary.needsFaceToFace`)
        : decision?.identity === EIdentityMethod.VERIFIED
          ? t(`${KEY}.summary.scanVerified`)
          : ""

    if (application.status === IApplicationsStatus.ACCEPTED) {
        return {
            headline: officer
                ? t(`${KEY}.summary.approvedBy`, {name: officer})
                : t(`${KEY}.summary.approvedAuto`),
            detail: found,
        }
    }
    if (application.status === IApplicationsStatus.REJECTED) {
        return {
            headline: officer
                ? t(`${KEY}.summary.rejectedBy`, {name: officer})
                : t(`${KEY}.summary.rejectedAuto`),
            detail: found,
        }
    }
    if (manual) {
        return {headline: t(`${KEY}.summary.typedByHand`), detail: identity}
    }
    return {headline: found || t(`${KEY}.summary.needsReview`), detail: identity}
}

const MINUTE = 60 * 1000
const HOUR = 60 * MINUTE
const DAY = 24 * HOUR

/** "3 hours", "9 days": how long ago the enrollment was sent. */
export function waitingTime(createdAt: string | null | undefined, now: Date, t: Translate): string {
    const sent = createdAt ? new Date(createdAt).getTime() : NaN
    if (Number.isNaN(sent)) {
        return ""
    }
    const elapsed = Math.max(0, now.getTime() - sent)
    if (elapsed >= DAY) {
        return t(`${KEY}.time.days`, {count: Math.floor(elapsed / DAY)})
    }
    if (elapsed >= HOUR) {
        return t(`${KEY}.time.hours`, {count: Math.floor(elapsed / HOUR)})
    }
    return t(`${KEY}.time.minutes`, {count: Math.max(1, Math.floor(elapsed / MINUTE))})
}

/** Lower case, without accents, hyphens or periods, as the rules compare names. */
export const comparable = (value: string): string =>
    value
        .toLowerCase()
        .replace(/-/g, " ")
        .replace(/\./g, "")
        .normalize("NFD")
        .replace(/[̀-ͯ]/g, "")
        .replace(/\s+/g, " ")
        .trim()

export const sameValue = (enrollment: string, registry: string): boolean =>
    comparable(enrollment) === comparable(registry)

/** A registry voter's value of a compared field. */
export function voterValue(voter: IRegistryVoter, field: string): string {
    const column = VOTER_COLUMNS[field]
    if (column) {
        return text(voter[column])
    }
    const value = voter.attributes?.[field]
    return text(Array.isArray(value) ? value[0] : value)
}

/**
 * Compares the enrollment with a registry voter on the compared fields, the
 * way the rules do: first and middle name together for the identity documents
 * that print them together.
 */
export function compareWithVoter(
    application: IApplication,
    voter: IRegistryVoter,
    comparedFields: string[]
): IComparedDetail[] {
    const data = applicantData(application)
    const joint =
        JOINT_NAME_DOCUMENTS.includes(data[ID_CARD_TYPE_FIELD] ?? "") &&
        comparedFields.includes(FIRST_NAME)
    const details: IComparedDetail[] = []
    comparedFields.forEach((field) => {
        if (joint && field === MIDDLE_NAME) {
            return
        }
        const fields = joint && field === FIRST_NAME ? [FIRST_NAME, MIDDLE_NAME] : [field]
        const enrollment = fields
            .map((name) => data[name] ?? "")
            .join(" ")
            .trim()
        const registry = fields
            .map((name) => voterValue(voter, name))
            .join(" ")
            .trim()
        details.push({fields, enrollment, registry, same: sameValue(enrollment, registry)})
    })
    return details
}

/** A voter is already enrolled when an attribute that enrolling sets is set. */
export const isAlreadyEnrolled = (voter: IRegistryVoter, unsetAttributes: string[]): boolean =>
    unsetAttributes.some((attribute) => voterValue(voter, attribute) !== "")

/**
 * The registry voters compared with the enrollment, each once: the closest
 * first, and those already enrolled, who can't be chosen, last.
 */
export function rankCandidates(
    application: IApplication,
    voters: IRegistryVoter[],
    comparedFields: string[],
    unsetAttributes: string[]
): ICandidate[] {
    const seen = new Set<string>()
    const candidates: ICandidate[] = []
    voters.forEach((voter) => {
        const id = voter.id ?? ""
        if (!id || seen.has(id)) {
            return
        }
        seen.add(id)
        const details = compareWithVoter(application, voter, comparedFields)
        candidates.push({
            voter,
            details,
            matching: details.filter((detail) => detail.same).length,
            alreadyEnrolled: isAlreadyEnrolled(voter, unsetAttributes),
        })
    })
    return candidates.sort(
        (a, b) => Number(a.alreadyEnrolled) - Number(b.alreadyEnrolled) || b.matching - a.matching
    )
}

/** A value as the review shows it: a date written out, and a dash for none. */
export function displayValue(value: string, language: string): string {
    if (!value) {
        return "—"
    }
    const date = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value)
    if (date) {
        const parsed = new Date(Date.UTC(Number(date[1]), Number(date[2]) - 1, Number(date[3])))
        if (parsed.toISOString().slice(0, 10) === value) {
            return parsed.toLocaleDateString(language, {
                day: "numeric",
                month: "short",
                year: "numeric",
                timeZone: "UTC",
            })
        }
    }
    return value
}

export type VoterFilter = Record<string, unknown>

const voterFilter = (data: Record<string, string>, fields: string[]): VoterFilter => {
    const filter: VoterFilter = {}
    const attributes: Record<string, string> = {}
    fields.forEach((field) => {
        const value = data[field]
        if (!value) {
            return
        }
        const column = VOTER_COLUMNS[field]
        if (column) {
            filter[column] = {IsLike: value}
        } else {
            attributes[field] = value
        }
    })
    return Object.keys(attributes).length > 0 ? {...filter, attributes} : filter
}

/**
 * The registry lookups that find the voters closest to the enrollment: one
 * with every compared field, and one leaving each field out, so a voter who
 * differs in one detail is found too.
 */
export function candidateFilters(
    application: IApplication,
    comparedFields: string[]
): VoterFilter[] {
    const data = applicantData(application)
    const fields = comparedFields.filter((field) => data[field])
    const lookups = [fields, ...fields.map((left) => fields.filter((field) => field !== left))]
    const filters: VoterFilter[] = []
    const seen = new Set<string>()
    lookups.forEach((lookup) => {
        const filter = voterFilter(data, lookup)
        const key = JSON.stringify(filter)
        if (Object.keys(filter).length > 0 && !seen.has(key)) {
            seen.add(key)
            filters.push(filter)
        }
    })
    return filters
}

/** The registry lookups of a free text search: by first name, last name and email. */
export function searchFilters(search: string): VoterFilter[] {
    const value = search.trim()
    if (!value) {
        return []
    }
    const words = value.split(/\s+/)
    const filters: VoterFilter[] = [
        {first_name: {IsLike: value}},
        {last_name: {IsLike: value}},
        {email: {IsLike: value}},
    ]
    if (words.length > 1) {
        filters.push({
            first_name: {IsLike: words.slice(0, -1).join(" ")},
            last_name: {IsLike: words[words.length - 1]},
        })
        filters.push({
            first_name: {IsLike: words[0]},
            last_name: {IsLike: words.slice(1).join(" ")},
        })
    }
    return filters
}

/** "Juan Dela Cruz", or the username of a voter without a name. */
export const voterName = (voter: IRegistryVoter): string =>
    [text(voter.first_name), voterValue(voter, MIDDLE_NAME), text(voter.last_name)]
        .filter((part) => part)
        .join(" ") || text(voter.username)

/** "the first name is “Juan Carlos” on the enrollment and “Juan” in the registry". */
export function differenceText(
    detail: IComparedDetail,
    t: Translate,
    fieldLabel: FieldLabel
): string {
    return t(`${KEY}.review.why.difference`, {
        field: joinList(
            detail.fields.map((field) => fieldLabel(field).toLowerCase()),
            t
        ),
        enrollment: detail.enrollment || "—",
        registry: detail.registry || "—",
    })
}

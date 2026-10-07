// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {canonicalize_slates_js, check_election_slates_js} from "sequent-core"
import {EMobileCandidateLists, ISlateProblem, SLATES_ANNOTATION} from "@sequentech/ui-core"
import {Sequent_Backend_Candidate, Sequent_Backend_Contest} from "@/gql/graphql"

/** The election form field holding the slate configuration as editable text. */
export const SLATES_FORM_FIELD = "slates_configuration" as const

type EntityAnnotations = Record<string, unknown> | null | undefined

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const parseObject = (text: string): Record<string, unknown> | undefined => {
    try {
        const parsed: unknown = JSON.parse(text)
        return isRecord(parsed) ? parsed : undefined
    } catch {
        return undefined
    }
}

const isProblemList = (value: unknown): value is Array<ISlateProblem> =>
    Array.isArray(value) &&
    value.every(
        (item) =>
            isRecord(item) &&
            typeof item.severity === "string" &&
            typeof item.code === "string" &&
            typeof item.path === "string" &&
            typeof item.message === "string"
    )

/** The stored slate configuration as text an administrator can edit. */
export const readSlatesConfiguration = (annotations: EntityAnnotations): string => {
    const stored = annotations?.[SLATES_ANNOTATION]
    if (typeof stored !== "string") {
        return ""
    }
    const parsed = parseObject(stored)
    return parsed ? JSON.stringify(parsed, null, 2) : stored
}

/**
 * The annotations to save for the edited slate configuration: its canonical
 * form, or no slate annotation at all when the field was cleared. Every other
 * annotation is kept as it is.
 */
export const writeSlatesConfiguration = (
    annotations: EntityAnnotations,
    text: string | null | undefined
): Record<string, unknown> => {
    const result: Record<string, unknown> = {...(annotations ?? {})}
    if (!text?.trim()) {
        delete result[SLATES_ANNOTATION]
        return result
    }
    result[SLATES_ANNOTATION] = canonicalize_slates_js(text)
    return result
}

/** The mobile candidate lists default the configuration text sets, if readable. */
export const getMobileCandidateLists = (
    text: string | null | undefined
): EMobileCandidateLists | undefined => {
    const parsed = text ? parseObject(text) : undefined
    if (!parsed) {
        return undefined
    }
    const value = parsed.mobile_candidate_lists
    if (value === undefined) {
        return EMobileCandidateLists.COLLAPSED
    }
    return Object.values(EMobileCandidateLists).find((option) => option === value)
}

/** The configuration text with its mobile candidate lists default changed. */
export const setMobileCandidateLists = (text: string, value: EMobileCandidateLists): string => {
    const parsed = parseObject(text)
    if (!parsed) {
        return text
    }
    const {version, mobile_candidate_lists: _previous, ...rest} = parsed
    return JSON.stringify({version, mobile_candidate_lists: value, ...rest}, null, 2)
}

/**
 * Everything that would stop the slate configuration from being published,
 * as the server will check it.
 */
export const checkSlatesConfiguration = (
    text: string,
    defaultLanguage: string,
    contests: Array<Sequent_Backend_Contest>,
    candidates: Array<Sequent_Backend_Candidate>
): Array<ISlateProblem> => {
    const problems: unknown = check_election_slates_js(
        text,
        defaultLanguage,
        JSON.stringify(contests),
        JSON.stringify(candidates)
    )
    if (!isProblemList(problems)) {
        throw new Error("Unexpected answer checking the slate configuration")
    }
    return problems
}

/** The problems a thrown slate check carries, when that is what was thrown. */
export const slatesProblemsFromError = (error: unknown): Array<ISlateProblem> | undefined =>
    isProblemList(error) ? error : undefined

/**
 * The problems that can be found without the election's contests and
 * candidates. The server still checks the rest before anything is published.
 */
export const checkSlatesStructure = (text: string): Array<ISlateProblem> => {
    try {
        canonicalize_slates_js(text)
        return []
    } catch (error) {
        const problems = slatesProblemsFromError(error)
        if (!problems) {
            throw error
        }
        return problems
    }
}

export const formatSlatesProblems = (problems: Array<ISlateProblem>): string =>
    problems.map((problem) => `${problem.path}: ${problem.message}`).join("\n")

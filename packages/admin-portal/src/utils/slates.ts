// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {canonicalize_slates_js, check_election_slates_js} from "sequent-core"
import {
    EMobileCandidateLists,
    ESlateProblemSeverity,
    ISlate,
    ISlateProblem,
    ISlatesConfig,
    SLATES_ANNOTATION,
    getSlateName,
} from "@sequentech/ui-core"
import {Sequent_Backend_Candidate, Sequent_Backend_Contest} from "@/gql/graphql"

/** The only slate configuration version the platform reads. */
export const SLATES_VERSION = 1

/** The election form fields of the Ballot Design slate settings. */
export const SLATES_ENABLED_FIELD = "slates_enabled" as const
export const SLATES_MOBILE_CANDIDATE_LISTS_FIELD = "slates_mobile_candidate_lists" as const

type EntityAnnotations = Record<string, unknown> | null | undefined

export interface ISlatesSettings {
    enabled: boolean
    mobileCandidateLists: EMobileCandidateLists
}

export interface ISlateFormValues {
    name: string
    alias: string
    description: string
    external_id: string
    /** One entry per picker by contest id; an empty string is an empty picker. */
    members: Record<string, Array<string>>
}

interface ISlateCandidate {
    id: string
    contest_id?: string | null
    presentation?: unknown
}

interface ISlateContest {
    id: string
    max_votes?: number | null
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

const isStringList = (value: unknown): value is Array<string> =>
    Array.isArray(value) && value.every((item) => typeof item === "string")

const isSlate = (value: unknown): value is ISlate =>
    isRecord(value) &&
    typeof value.id === "string" &&
    isRecord(value.name) &&
    Object.values(value.name).every((name) => typeof name === "string") &&
    isRecord(value.members) &&
    Object.values(value.members).every(isStringList)

const isSlatesConfig = (value: unknown): value is ISlatesConfig =>
    isRecord(value) &&
    typeof value.version === "number" &&
    Array.isArray(value.slates) &&
    value.slates.every(isSlate)

const isProblemList = (value: unknown): value is Array<ISlateProblem> =>
    Array.isArray(value) &&
    value.every(
        (item) =>
            isRecord(item) &&
            Object.values(ESlateProblemSeverity).some((severity) => severity === item.severity) &&
            typeof item.code === "string" &&
            typeof item.path === "string" &&
            typeof item.message === "string"
    )

export const hasSlatesAnnotation = (annotations: EntityAnnotations): boolean =>
    annotations?.[SLATES_ANNOTATION] !== undefined

/** The stored slate configuration, when there is one that can be read. */
export const readSlatesConfig = (annotations: EntityAnnotations): ISlatesConfig | undefined => {
    const stored = annotations?.[SLATES_ANNOTATION]
    if (typeof stored !== "string") {
        return undefined
    }
    try {
        const parsed: unknown = JSON.parse(stored)
        return isSlatesConfig(parsed) ? parsed : undefined
    } catch {
        return undefined
    }
}

/**
 * The annotations with the canonical form of `config`, or without the slate
 * annotation when there is no configuration. Every other annotation is kept.
 */
export const writeSlatesConfig = (
    annotations: EntityAnnotations,
    config: ISlatesConfig | undefined
): Record<string, unknown> => {
    const result: Record<string, unknown> = {...(annotations ?? {})}
    if (!config) {
        delete result[SLATES_ANNOTATION]
        return result
    }
    result[SLATES_ANNOTATION] = canonicalize_slates_js(JSON.stringify(config))
    return result
}

/** The annotations after saving the Ballot Design slate settings. */
export const applySlatesSettings = (
    annotations: EntityAnnotations,
    settings: ISlatesSettings
): Record<string, unknown> => {
    if (!settings.enabled) {
        return writeSlatesConfig(annotations, undefined)
    }
    const config = readSlatesConfig(annotations)
    if (!config && hasSlatesAnnotation(annotations)) {
        return {...annotations}
    }
    return writeSlatesConfig(annotations, {
        version: config?.version ?? SLATES_VERSION,
        mobile_candidate_lists: settings.mobileCandidateLists,
        slates: config?.slates ?? [],
    })
}

/** The configuration with `slate` in place of the one with its id, or appended. */
export const putSlate = (config: ISlatesConfig, slate: ISlate): ISlatesConfig => {
    const index = config.slates.findIndex(({id}) => id === slate.id)
    const slates =
        index === -1
            ? [...config.slates, slate]
            : config.slates.map((current, position) => (position === index ? slate : current))
    return {...config, slates}
}

export const removeSlate = (config: ISlatesConfig, slateId: string): ISlatesConfig => ({
    ...config,
    slates: config.slates.filter(({id}) => id !== slateId),
})

const UNSUITABLE_MEMBER_FLAGS = [
    "is_disabled",
    "is_write_in",
    "is_explicit_blank",
    "is_explicit_invalid",
    "is_category_list",
] as const

/** Whether a candidate can be a slate member, as sequent-core decides it. */
export const isEligibleSlateMember = (candidate: ISlateCandidate): boolean => {
    const presentation = candidate.presentation
    return !(
        isRecord(presentation) &&
        UNSUITABLE_MEMBER_FLAGS.some((flag) => presentation[flag] === true)
    )
}

/** The candidates of a contest the slate `slateId` can list. */
export const slateCandidateChoices = <T extends ISlateCandidate>(
    candidates: Array<T>,
    contestId: string,
    config: ISlatesConfig | undefined,
    slateId: string | undefined
): Array<T> => {
    const onOtherSlates = new Set(
        (config?.slates ?? [])
            .filter(({id}) => id !== slateId)
            .flatMap((slate) => slate.members[contestId] ?? [])
    )
    return candidates.filter(
        (candidate) =>
            candidate.contest_id === contestId &&
            isEligibleSlateMember(candidate) &&
            !onOtherSlates.has(candidate.id)
    )
}

/** The form values of a slate: one picker per seat of each contest. */
export const slateFormValues = (
    slate: ISlate | undefined,
    contests: Array<ISlateContest>,
    defaultLanguage: string
): ISlateFormValues => {
    const members: Record<string, Array<string>> = {}
    for (const contest of contests) {
        const current = slate?.members[contest.id] ?? []
        const seats = Math.max(contest.max_votes ?? 1, current.length, 1)
        members[contest.id] = Array.from({length: seats}, (_, index) => current[index] ?? "")
    }
    return {
        name: slate ? getSlateName(slate, defaultLanguage, defaultLanguage) : "",
        alias: slate?.alias ?? "",
        description: slate?.description ?? "",
        external_id: slate?.external_id ?? "",
        members,
    }
}

/** The slate the form describes, keeping its names in other languages. */
export const slateFromForm = (
    id: string,
    values: ISlateFormValues,
    defaultLanguage: string,
    previous: ISlate | undefined
): ISlate => {
    const members: Record<string, Array<string>> = {}
    for (const [contestId, candidateIds] of Object.entries(values.members ?? {})) {
        const chosen = Array.from(new Set((candidateIds ?? []).filter(Boolean)))
        if (chosen.length > 0) {
            members[contestId] = chosen
        }
    }
    const slate: ISlate = {
        id,
        name: {...(previous?.name ?? {}), [defaultLanguage]: (values.name ?? "").trim()},
        members,
    }
    const alias = (values.alias ?? "").trim()
    const description = (values.description ?? "").trim()
    const externalId = (values.external_id ?? "").trim()
    if (alias) {
        slate.alias = alias
    }
    if (description) {
        slate.description = description
    }
    if (externalId) {
        slate.external_id = externalId
    }
    return slate
}

/** The problems of the slate configuration as the server will check it. */
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

export const formatSlatesProblems = (problems: Array<ISlateProblem>): string =>
    problems.map((problem) => `${problem.path}: ${problem.message}`).join("\n")

// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {IBallotStyle as IElectionEml, IContest, ICandidate} from "@sequentech/ui-core"
import {wrapTranslation, stripMarkers} from "./markers"
import {OverridesByLanguage} from "./i18n"

export class UnrecognizedElectionFormatError extends Error {}

export interface ContentFieldRef {
    key: string
    group: string
    fieldLabel: string
    getOriginal: (lang: string) => string
    getCurrent: (lang: string) => string
    setValue: (lang: string, value: string) => void
}

export interface UploadedBallotStyle {
    id: string
    election_id: string
    ballot_eml: IElectionEml
}

export interface UploadedElectionEvent {
    fileName: string
    format: "election-event-schema" | "ballot-styles"
    raw: unknown
    tenantId: string
    electionEventId: string
    ballotStyles: UploadedBallotStyle[]
    languages: string[]
    fieldRefs: Map<string, ContentFieldRef>
    electionEventPresentations: PresentationLike[]
}

type PresentationLike = {i18n?: Record<string, Record<string, string>>; [key: string]: unknown}

const DEFAULT_LANGUAGES = ["en", "es", "cat", "fr", "tl", "gl", "nl", "eu"]

export const CONTENT_KEY_PREFIX = "content::"

export const contentKey = (entityType: string, entityId: string, field: string): string =>
    `${CONTENT_KEY_PREFIX}${entityType}::${entityId}::${field}`

export const isContentKey = (key: string): boolean => key.startsWith(CONTENT_KEY_PREFIX)

interface PresentationAccessor {
    read: (lang: string) => Record<string, string> | undefined
    ensureBucket: (lang: string) => Record<string, string>
}

const directPresentationAccessor = (holder: PresentationLike): PresentationAccessor => ({
    read: (lang) => holder.i18n?.[lang],
    ensureBucket: (lang) => {
        if (!holder.i18n) {
            holder.i18n = {}
        }
        if (!holder.i18n[lang]) {
            holder.i18n[lang] = {}
        }
        return holder.i18n[lang]
    },
})

const nestedPresentationAccessor = (
    holder: {presentation?: PresentationLike | null} & Record<string, unknown>
): PresentationAccessor => ({
    read: (lang) => (holder.presentation as PresentationLike | undefined)?.i18n?.[lang],
    ensureBucket: (lang) => {
        if (!holder.presentation) {
            holder.presentation = {}
        }
        const presentation = holder.presentation as PresentationLike
        if (!presentation.i18n) {
            presentation.i18n = {}
        }
        if (!presentation.i18n[lang]) {
            presentation.i18n[lang] = {}
        }
        return presentation.i18n[lang]
    },
})

const collectLanguagesFromI18n = (
    i18n: Record<string, Record<string, string>> | undefined,
    into: Set<string>
): void => {
    if (i18n && typeof i18n === "object") {
        Object.keys(i18n).forEach((lang) => into.add(lang))
    }
}

interface FieldSpec {
    field: string
    label: string
}

const buildFieldRefs = (
    accessors: PresentationAccessor[],
    flatHolder: Record<string, unknown>,
    denormalizedHolders: Record<string, unknown>[] | null,
    entityType: string,
    entityId: string,
    group: string,
    specs: FieldSpec[],
    languages: string[],
    fieldRefs: Map<string, ContentFieldRef>
): void => {
    if (accessors.length === 0) {
        return
    }
    specs.forEach(({field, label}) => {
        const key = contentKey(entityType, entityId, field)
        const i18nField = `${field}_i18n`

        const getCurrent = (lang: string): string => {
            if (denormalizedHolders) {
                for (const holder of denormalizedHolders) {
                    const dict = holder[i18nField] as Record<string, string> | undefined
                    if (dict && typeof dict[lang] === "string" && dict[lang].length > 0) {
                        return dict[lang]
                    }
                }
            }
            const bucketValue = accessors[0].read(lang)?.[field]
            if (typeof bucketValue === "string" && bucketValue.length > 0) {
                return bucketValue
            }
            const flat = flatHolder[field]
            return typeof flat === "string" ? flat : ""
        }

        const setValue = (lang: string, value: string): void => {
            accessors.forEach((accessor) => {
                accessor.ensureBucket(lang)[field] = value
            })
            denormalizedHolders?.forEach((holder) => {
                const dict = (holder[i18nField] as Record<string, string> | undefined) || {}
                dict[lang] = value
                holder[i18nField] = dict
            })
        }

        const original: Record<string, string> = {}
        languages.forEach((lang) => {
            original[lang] = getCurrent(lang)
        })

        fieldRefs.set(key, {
            key,
            group,
            fieldLabel: label,
            getOriginal: (lang) => (lang in original ? original[lang] : getCurrent(lang)),
            getCurrent,
            setValue,
        })
    })
}

const ELECTION_EVENT_FIELDS: FieldSpec[] = [
    {field: "name", label: "Name"},
    {field: "alias", label: "Alias"},
    {field: "description", label: "Description"},
    {field: "materialsTitle", label: "Materials title"},
    {field: "materialsSubtitle", label: "Materials subtitle"},
]

const ELECTION_FIELDS: FieldSpec[] = [
    {field: "name", label: "Name"},
    {field: "alias", label: "Alias"},
    {field: "description", label: "Description"},
    {field: "security_confirmation_html", label: "Security confirmation"},
]

const CONTEST_CANDIDATE_FIELDS: FieldSpec[] = [
    {field: "name", label: "Name"},
    {field: "description", label: "Description"},
    {field: "alias", label: "Alias"},
]

const asRecord = (value: unknown): Record<string, unknown> => {
    if (typeof value === "string") {
        try {
            const parsed = JSON.parse(value) as unknown
            if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
                return parsed as Record<string, unknown>
            }
        } catch {
            return {}
        }
    }
    return (value && typeof value === "object" && !Array.isArray(value)
        ? (value as Record<string, unknown>)
        : {}) as Record<string, unknown>
}

const asArray = <T extends Record<string, unknown>>(value: unknown): T[] =>
    Array.isArray(value) ? (value as T[]) : []

const normalizePresentation = (value: unknown): PresentationLike => asRecord(value) as PresentationLike

const parseElectionEventSchema = (
    obj: Record<string, unknown>,
    languages: Set<string>,
    fieldRefs: Map<string, ContentFieldRef>
): {ballotStyles: UploadedBallotStyle[]; electionEventPresentations: PresentationLike[]} => {
    const electionEvent = asRecord(obj.election_event)
    if (!electionEvent.id) {
        throw new UnrecognizedElectionFormatError("election_event.id is missing from that file.")
    }
    const electionEventPresentation = normalizePresentation(electionEvent.presentation)
    electionEvent.presentation = electionEventPresentation
    collectLanguagesFromI18n(electionEventPresentation.i18n, languages)

    const eventId = String(electionEvent.id)
    const tenantId = String(electionEvent.tenant_id || obj.tenant_id || "")

    const contestsByElection = new Map<string, Record<string, unknown>[]>()
    asArray<Record<string, unknown>>(obj.contests).forEach((contest) => {
        const presentation = normalizePresentation(contest.presentation)
        contest.presentation = presentation
        collectLanguagesFromI18n(presentation.i18n, languages)
        const list = contestsByElection.get(String(contest.election_id)) || []
        list.push(contest)
        contestsByElection.set(String(contest.election_id), list)
    })

    const candidatesByContest = new Map<string, Record<string, unknown>[]>()
    asArray<Record<string, unknown>>(obj.candidates).forEach((candidate) => {
        const presentation = normalizePresentation(candidate.presentation)
        candidate.presentation = presentation
        collectLanguagesFromI18n(presentation.i18n, languages)
        const list = candidatesByContest.get(String(candidate.contest_id)) || []
        list.push(candidate)
        candidatesByContest.set(String(candidate.contest_id), list)
    })

    buildFieldRefs(
        [nestedPresentationAccessor(electionEvent)],
        electionEvent,
        null,
        "election_event",
        eventId,
        "Election event",
        ELECTION_EVENT_FIELDS,
        Array.from(languages),
        fieldRefs
    )

    const ballotStyles: UploadedBallotStyle[] = asArray<Record<string, unknown>>(obj.elections).map(
        (election) => {
            const electionPresentation = normalizePresentation(election.presentation)
            election.presentation = electionPresentation
            collectLanguagesFromI18n(electionPresentation.i18n, languages)
            const electionId = String(election.id)
            const electionLabel =
                Object.values(electionPresentation.i18n || {})[0]?.name || electionId

            buildFieldRefs(
                [nestedPresentationAccessor(election)],
                election,
                null,
                "election",
                electionId,
                `Election: ${electionLabel}`,
                ELECTION_FIELDS,
                Array.from(languages),
                fieldRefs
            )

            const contests: IContest[] = (contestsByElection.get(electionId) || []).map((contest) => {
                const contestId = String(contest.id)
                const contestPresentation = contest.presentation as PresentationLike
                const contestLabel =
                    Object.values(contestPresentation.i18n || {})[0]?.name ||
                    (typeof contest.description === "string" ? contest.description : contestId)

                buildFieldRefs(
                    [nestedPresentationAccessor(contest)],
                    contest,
                    null,
                    "contest",
                    contestId,
                    `Contest: ${contestLabel}`,
                    CONTEST_CANDIDATE_FIELDS,
                    Array.from(languages),
                    fieldRefs
                )

                const candidates: ICandidate[] = (candidatesByContest.get(contestId) || []).map(
                    (candidate) => {
                        const candidateId = String(candidate.id)
                        const candidatePresentation = candidate.presentation as PresentationLike
                        const candidateLabel =
                            Object.values(candidatePresentation.i18n || {})[0]?.name ||
                            (typeof candidate.description === "string"
                                ? candidate.description
                                : candidateId)

                        buildFieldRefs(
                            [nestedPresentationAccessor(candidate)],
                            candidate,
                            null,
                            "candidate",
                            candidateId,
                            `Candidate: ${candidateLabel}`,
                            CONTEST_CANDIDATE_FIELDS,
                            Array.from(languages),
                            fieldRefs
                        )

                        return {
                            id: candidateId,
                            tenant_id: String(candidate.tenant_id || tenantId || electionEvent.tenant_id || ""),
                            election_event_id: eventId,
                            election_id: electionId,
                            contest_id: contestId,
                            name: candidateLabel,
                            description: candidate.description as string | undefined,
                            candidate_type: (candidate.type as string | undefined) || candidate.candidate_type as string | undefined,
                            presentation: candidatePresentation,
                        } as unknown as ICandidate
                    }
                )

                return {
                    id: contestId,
                    tenant_id: String(contest.tenant_id || tenantId || electionEvent.tenant_id || ""),
                    election_event_id: eventId,
                    election_id: electionId,
                    name: contestLabel,
                    description: contest.description as string | undefined,
                    max_votes: Number(contest.max_votes ?? 1),
                    min_votes: Number(contest.min_votes ?? 0),
                    winning_candidates_num: Number(contest.winning_candidates_num ?? 1),
                    voting_type: (contest.voting_type as string | undefined) || "plurality",
                    is_encrypted: contest.is_encrypted !== false,
                    candidates,
                    presentation: contestPresentation,
                } as unknown as IContest
            })

            const ballotEml = {
                id: electionId,
                tenant_id: String(election.tenant_id || tenantId || electionEvent.tenant_id || ""),
                election_event_id: eventId,
                election_id: electionId,
                area_id: "loc-studio-area",
                public_key: electionEvent.public_key
                    ? {
                          public_key: String(electionEvent.public_key),
                          is_demo: false,
                      }
                    : undefined,
                contests,
                election_event_presentation: electionEventPresentation,
                election_presentation: electionPresentation,
            } as unknown as IElectionEml

            return {id: electionId, election_id: electionId, ballot_eml: ballotEml}
        }
    )

    return {ballotStyles, electionEventPresentations: [electionEventPresentation]}
}

const normalizeBallotStyleEntry = (
    entry: Record<string, unknown>,
    languages: Set<string>,
    fieldRefs: Map<string, ContentFieldRef>,
    eventPresentationsByEventId: Map<string, PresentationLike>
): UploadedBallotStyle => {
    const ballotEml = (
        "ballot_eml" in entry ? entry.ballot_eml : entry
    ) as Record<string, unknown> & IElectionEml

    const electionEventPresentation = normalizePresentation(ballotEml.election_event_presentation)
    ballotEml.election_event_presentation = electionEventPresentation as never
    const electionPresentation = normalizePresentation(ballotEml.election_presentation)
    ballotEml.election_presentation = electionPresentation as never

    collectLanguagesFromI18n(electionEventPresentation.i18n, languages)
    collectLanguagesFromI18n(electionPresentation.i18n, languages)

    const eventId = String(ballotEml.election_event_id || "event")
    const isFirstForEvent = !eventPresentationsByEventId.has(eventId)
    const accessorsForEvent: PresentationAccessor[] = []
    eventPresentationsByEventId.set(eventId, electionEventPresentation)

    if (isFirstForEvent) {
        accessorsForEvent.push(directPresentationAccessor(electionEventPresentation))
        buildFieldRefs(
            accessorsForEvent,
            electionEventPresentation,
            null,
            "election_event",
            eventId,
            "Election event",
            ELECTION_EVENT_FIELDS,
            Array.from(languages),
            fieldRefs
        )
    }

    const electionId = String(ballotEml.election_id || ballotEml.id)
    const electionLabel = Object.values(electionPresentation.i18n || {})[0]?.name || electionId
    buildFieldRefs(
        [directPresentationAccessor(electionPresentation)],
        electionPresentation,
        null,
        "election",
        electionId,
        `Election: ${electionLabel}`,
        ELECTION_FIELDS,
        Array.from(languages),
        fieldRefs
    )

    ;(ballotEml.contests || []).forEach((contest) => {
        const contestRecord = contest as unknown as Record<string, unknown> & IContest
        const contestPresentation = normalizePresentation(contestRecord.presentation)
        contestRecord.presentation = contestPresentation as never
        collectLanguagesFromI18n(contestPresentation.i18n, languages)
        const contestLabel = contestRecord.name || contest.id

        buildFieldRefs(
            [nestedPresentationAccessor(contestRecord as {presentation?: PresentationLike | null} & Record<string, unknown>)],
            contestRecord,
            [contestRecord],
            "contest",
            contest.id,
            `Contest: ${contestLabel}`,
            CONTEST_CANDIDATE_FIELDS,
            Array.from(languages),
            fieldRefs
        )

        ;(contest.candidates || []).forEach((candidate) => {
            const candidateRecord = candidate as unknown as Record<string, unknown> & ICandidate
            const candidatePresentation = normalizePresentation(candidateRecord.presentation)
            candidateRecord.presentation = candidatePresentation as never
            collectLanguagesFromI18n(candidatePresentation.i18n, languages)
            const candidateLabel = candidateRecord.name || candidate.id

            buildFieldRefs(
                [
                    nestedPresentationAccessor(
                        candidateRecord as {presentation?: PresentationLike | null} & Record<string, unknown>
                    ),
                ],
                candidateRecord,
                [candidateRecord],
                "candidate",
                candidate.id,
                `Candidate: ${candidateLabel}`,
                CONTEST_CANDIDATE_FIELDS,
                Array.from(languages),
                fieldRefs
            )
        })
    })

    return {
        id: String(ballotEml.id || electionId),
        election_id: electionId,
        ballot_eml: ballotEml as unknown as IElectionEml,
    }
}

export const parseUploadedElectionEvent = (raw: unknown, fileName: string): UploadedElectionEvent => {
    if (!raw || typeof raw !== "object") {
        throw new UnrecognizedElectionFormatError("File does not contain a JSON object.")
    }
    const obj = raw as Record<string, unknown>
    const languages = new Set<string>(DEFAULT_LANGUAGES)
    const fieldRefs = new Map<string, ContentFieldRef>()

    let format: UploadedElectionEvent["format"]
    let ballotStyles: UploadedBallotStyle[] = []
    let electionEventPresentations: PresentationLike[] = []

    if (
        obj.election_event &&
        Array.isArray(obj.elections)
    ) {
        format = "election-event-schema"
        const result = parseElectionEventSchema(obj, languages, fieldRefs)
        ballotStyles = result.ballotStyles
        electionEventPresentations = result.electionEventPresentations
    } else if (Array.isArray(obj.ballot_styles)) {
        format = "ballot-styles"
        const eventPresentationsByEventId = new Map<string, PresentationLike>()
        ballotStyles = (obj.ballot_styles as Record<string, unknown>[]).map((entry) =>
            normalizeBallotStyleEntry(entry, languages, fieldRefs, eventPresentationsByEventId)
        )
        electionEventPresentations = Array.from(eventPresentationsByEventId.values())
    } else if (obj.ballot_design && typeof obj.ballot_design === "object") {
        format = "ballot-styles"
        const design = obj.ballot_design as Record<string, unknown>
        const styles = asArray<Record<string, unknown>>(design.ballot_styles)
        const eventPresentationsByEventId = new Map<string, PresentationLike>()
        ballotStyles = styles.map((entry) =>
            normalizeBallotStyleEntry(entry, languages, fieldRefs, eventPresentationsByEventId)
        )
        electionEventPresentations = Array.from(eventPresentationsByEventId.values())
    } else if (Array.isArray(obj.contests) && typeof obj.id === "string") {
        format = "ballot-styles"
        const eventPresentationsByEventId = new Map<string, PresentationLike>()
        ballotStyles = [normalizeBallotStyleEntry(obj, languages, fieldRefs, eventPresentationsByEventId)]
        electionEventPresentations = Array.from(eventPresentationsByEventId.values())
    } else if (Array.isArray(raw)) {
        format = "ballot-styles"
        const eventPresentationsByEventId = new Map<string, PresentationLike>()
        ballotStyles = (raw as Record<string, unknown>[]).map((entry) =>
            normalizeBallotStyleEntry(entry, languages, fieldRefs, eventPresentationsByEventId)
        )
        electionEventPresentations = Array.from(eventPresentationsByEventId.values())
    } else {
        throw new UnrecognizedElectionFormatError(
            "Unrecognized election JSON. Expected an election event export " +
                "(election_event / elections / contests / candidates) or a ballot style export " +
                "(ballot_styles, or a single ballot style with a contests array)."
        )
    }

    if (ballotStyles.length === 0) {
        throw new UnrecognizedElectionFormatError("No elections were found in that file.")
    }

    const totalContests = ballotStyles.reduce(
        (count, ballotStyle) => count + (ballotStyle.ballot_eml.contests?.length || 0),
        0
    )
    if (totalContests === 0) {
        throw new UnrecognizedElectionFormatError(
            "That export has no ballot contests. Re-export with publications enabled, or use a publications JSON that includes ballot_styles."
        )
    }

    return {
        fileName,
        format,
        raw,
        tenantId: String(ballotStyles[0].ballot_eml.tenant_id || "loc-studio-tenant"),
        electionEventId: String(ballotStyles[0].ballot_eml.election_event_id || "loc-studio-event"),
        ballotStyles,
        languages: Array.from(languages),
        fieldRefs,
        electionEventPresentations,
    }
}

const wrapField = (
    fieldRefs: Map<string, ContentFieldRef>,
    key: string,
    lang: string
): string | undefined => {
    const ref = fieldRefs.get(key)
    if (!ref) {
        return undefined
    }
    const value = ref.getCurrent(lang)
    if (!value) {
        return undefined
    }
    return wrapTranslation(key, value)
}

const setDirectI18n = (
    presentation: PresentationLike,
    lang: string,
    field: string,
    value: string | undefined
): void => {
    if (value === undefined) {
        return
    }
    if (!presentation.i18n) {
        presentation.i18n = {}
    }
    if (!presentation.i18n[lang]) {
        presentation.i18n[lang] = {}
    }
    presentation.i18n[lang][field] = value
}

const CONTEST_TAG_FIELDS = ["name", "description", "alias"] as const

export const buildPreviewBallotStyles = (
    uploaded: UploadedElectionEvent,
    language: string
): UploadedBallotStyle[] =>
    uploaded.ballotStyles.map((bs) => {
        const eml = JSON.parse(JSON.stringify(bs.ballot_eml)) as IElectionEml
        const eventId = String(eml.election_event_id || uploaded.electionEventId)

        const eventPresentation = (eml.election_event_presentation || {}) as PresentationLike
        eml.election_event_presentation = eventPresentation as never
        ELECTION_EVENT_FIELDS.forEach(({field}) => {
            setDirectI18n(
                eventPresentation,
                language,
                field,
                wrapField(uploaded.fieldRefs, contentKey("election_event", eventId, field), language)
            )
        })

        const electionPresentation = (eml.election_presentation || {}) as PresentationLike
        eml.election_presentation = electionPresentation as never
        ELECTION_FIELDS.forEach(({field}) => {
            setDirectI18n(
                electionPresentation,
                language,
                field,
                wrapField(uploaded.fieldRefs, contentKey("election", eml.election_id, field), language)
            )
        })

        eml.contests = (eml.contests || []).map((contest) => {
            const contestPresentation = (contest.presentation || {}) as PresentationLike
            CONTEST_TAG_FIELDS.forEach((field) => {
                const wrapped = wrapField(
                    uploaded.fieldRefs,
                    contentKey("contest", contest.id, field),
                    language
                )
                if (wrapped === undefined) {
                    return
                }
                setDirectI18n(contestPresentation, language, field, wrapped)
                const dictField = `${field}_i18n` as "name_i18n" | "description_i18n" | "alias_i18n"
                contest[dictField] = {...contest[dictField], [language]: wrapped}
            })
            contest.presentation = contestPresentation as never

            contest.candidates = (contest.candidates || []).map((candidate) => {
                const candidatePresentation = (candidate.presentation || {}) as PresentationLike
                CONTEST_TAG_FIELDS.forEach((field) => {
                    const wrapped = wrapField(
                        uploaded.fieldRefs,
                        contentKey("candidate", candidate.id, field),
                        language
                    )
                    if (wrapped === undefined) {
                        return
                    }
                    setDirectI18n(candidatePresentation, language, field, wrapped)
                    const dictField = `${field}_i18n` as "name_i18n" | "description_i18n" | "alias_i18n"
                    candidate[dictField] = {...candidate[dictField], [language]: wrapped}
                })
                candidate.presentation = candidatePresentation as never
                return candidate
            })
            return contest
        })

        return {...bs, ballot_eml: eml}
    })

export const applyAppOverridesToUploaded = (
    uploaded: UploadedElectionEvent,
    overrides: OverridesByLanguage
): void => {
    Object.entries(overrides).forEach(([lang, values]) => {
        if (Object.keys(values).length === 0) {
            return
        }
        uploaded.electionEventPresentations.forEach((presentation) => {
            if (!presentation.i18n) {
                presentation.i18n = {}
            }
            presentation.i18n[lang] = {...presentation.i18n[lang], ...values}
        })
    })
}

export const resetAllContentFields = (uploaded: UploadedElectionEvent): void => {
    uploaded.fieldRefs.forEach((ref) => {
        uploaded.languages.forEach((lang) => {
            ref.setValue(lang, ref.getOriginal(lang))
        })
    })
}

const TRANSLATABLE_FIELDS = ["name", "description", "alias", "materialsTitle", "materialsSubtitle"] as const

const stripMarkersFromPresentation = (presentation: PresentationLike | undefined): void => {
    if (!presentation?.i18n) {
        return
    }
    Object.keys(presentation.i18n).forEach((lang) => {
        const bucket = presentation.i18n?.[lang]
        if (!bucket) {
            return
        }
        Object.keys(bucket).forEach((field) => {
            bucket[field] = stripMarkers(bucket[field] || "")
        })
    })
}

const syncDenormalizedFields = (holder: Record<string, unknown>, presentation: PresentationLike | undefined): void => {
    if (!presentation?.i18n) {
        return
    }
    TRANSLATABLE_FIELDS.forEach((field) => {
        const dict: Record<string, string> = {}
        Object.entries(presentation.i18n || {}).forEach(([lang, values]) => {
            const value = values[field]
            if (typeof value === "string" && value.length > 0) {
                dict[lang] = stripMarkers(value)
            }
        })
        if (Object.keys(dict).length > 0) {
            holder[`${field}_i18n`] = dict
        }
    })
}

const prepareRawForExport = (uploaded: UploadedElectionEvent): unknown => {
    const raw = uploaded.raw
    if (!raw || typeof raw !== "object") {
        return raw
    }

    if (uploaded.format === "election-event-schema") {
        const obj = raw as Record<string, unknown>
        const electionEvent = obj.election_event as Record<string, unknown> | undefined
        if (electionEvent?.presentation) {
            stripMarkersFromPresentation(electionEvent.presentation as PresentationLike)
        }
        ;(obj.elections as Record<string, unknown>[] | undefined)?.forEach((election) => {
            stripMarkersFromPresentation(election.presentation as PresentationLike)
        })
        ;(obj.contests as Record<string, unknown>[] | undefined)?.forEach((contest) => {
            stripMarkersFromPresentation(contest.presentation as PresentationLike)
        })
        ;(obj.candidates as Record<string, unknown>[] | undefined)?.forEach((candidate) => {
            stripMarkersFromPresentation(candidate.presentation as PresentationLike)
        })
        return obj
    }

    const syncBallotEml = (ballotEml: Record<string, unknown>): void => {
        const eventPresentation = ballotEml.election_event_presentation as PresentationLike | undefined
        const electionPresentation = ballotEml.election_presentation as PresentationLike | undefined
        stripMarkersFromPresentation(eventPresentation)
        stripMarkersFromPresentation(electionPresentation)
        ;(ballotEml.contests as Record<string, unknown>[] | undefined)?.forEach((contest) => {
            const contestPresentation = contest.presentation as PresentationLike | undefined
            stripMarkersFromPresentation(contestPresentation)
            syncDenormalizedFields(contest, contestPresentation)
            ;(contest.candidates as Record<string, unknown>[] | undefined)?.forEach((candidate) => {
                const candidatePresentation = candidate.presentation as PresentationLike | undefined
                stripMarkersFromPresentation(candidatePresentation)
                syncDenormalizedFields(candidate, candidatePresentation)
            })
        })
    }

    const obj = raw as Record<string, unknown>
    if (Array.isArray(obj.ballot_styles)) {
        ;(obj.ballot_styles as Record<string, unknown>[]).forEach((entry) => {
            const ballotEml = ("ballot_eml" in entry ? entry.ballot_eml : entry) as Record<string, unknown>
            syncBallotEml(ballotEml)
        })
        return obj
    }
    if (Array.isArray(raw)) {
        ;(raw as Record<string, unknown>[]).forEach((entry) => {
            const ballotEml = ("ballot_eml" in entry ? entry.ballot_eml : entry) as Record<string, unknown>
            syncBallotEml(ballotEml)
        })
        return raw
    }
    if (Array.isArray(obj.contests)) {
        syncBallotEml(obj)
        return obj
    }
    return raw
}

export const prepareUploadedEventForExport = (
    uploaded: UploadedElectionEvent,
    overrides: OverridesByLanguage
): unknown => {
    applyAppOverridesToUploaded(uploaded, overrides)
    return prepareRawForExport(uploaded)
}

export const exportUploadedElectionEvent = (
    uploaded: UploadedElectionEvent,
    overrides: OverridesByLanguage
): {fileName: string; content: string} => {
    const prepared = prepareUploadedEventForExport(uploaded, overrides)
    const content = JSON.stringify(prepared, null, 2)
    const base = uploaded.fileName.replace(/\.(localized\.)?json$/i, "")
    return {fileName: `${base}.localized.json`, content}
}

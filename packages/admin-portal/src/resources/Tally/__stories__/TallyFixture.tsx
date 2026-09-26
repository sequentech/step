// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic data of a tally ceremony: the results that ResultsDataLoader reads
// from the tally's SQLite export into `tallyQueryData` (JSON columns stay
// strings there, as sql.js returns them), the tally session and its execution
// at each workflow step, and the tally screen's context.
import React, {useContext, useState, type PropsWithChildren} from "react"
import {createStore, Provider as AtomProvider} from "jotai"
import type {
    GetTallyDataQuery,
    Sequent_Backend_Results_Event,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tally_Session_Execution,
} from "@/gql/graphql"
import {tallyQueryData} from "@/atoms/tally-candidates"
import {ElectionEventTallyContext} from "@/providers/ElectionEventTallyProvider"
import {
    ITallyElectionStatus,
    ITallyTrusteeStatus,
    type ITallyCeremonyStatus,
} from "@/types/ceremonies"
import type {IResultDocuments} from "@/types/results"
import type {IMiruTransmissionPackageData} from "@/types/miru"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {
    FIXED_TIME,
    STORY_IDS,
    areaRecords,
    candidateRecords,
    contestRecord,
    electionPresentation,
    electionRecord,
    eventPresentation,
    storyId,
    tallySessionRecord,
    trusteeRecords,
    type StoryRecord,
} from "@/__stories__/fixtures"
import {EStoryWorkflow} from "../../../../../ui-essentials/.storybook/globals"

export const TALLY_IDS = {
    resultsEvent: storyId(9, 1),
    execution: storyId(9, 2),
    deputyCandidate: storyId(6, 3),
    secondDeputyCandidate: storyId(6, 4),
} as const

/** A results file of the tally, e.g. `documentId(12)`. */
export const documentId = (index: number) =>
    `d0c00000-0000-4000-8000-${String(index).padStart(12, "0")}`

/** The files the tally generated at each level of the results. */
export const RESULT_DOCUMENTS = {
    event: {json: documentId(1), pdf: documentId(2), html: documentId(3), tar_gz: documentId(4)},
    election: {
        json: documentId(11),
        pdf: documentId(12),
        html: documentId(13),
        tar_gz: documentId(14),
        all_areas_html: documentId(15),
        all_areas_json: documentId(16),
    },
    electionArea: {json: documentId(21), pdf: documentId(22)},
    contest: {json: documentId(31), pdf: documentId(32), html: documentId(33)},
    areaContest: {json: documentId(41), html: documentId(43)},
} satisfies Record<string, IResultDocuments>

const text = (value: unknown) => JSON.stringify(value)
const scope = {tenant_id: TENANT_ID, election_event_id: EVENT_ID}
const eventResults = {...scope, results_event_id: TALLY_IDS.resultsEvent}
const [north, south] = areaRecords()

export const COUNCIL_ELECTION = electionRecord(EStoryWorkflow.RESULTS)
export const DEPUTY_ELECTION = electionRecord(EStoryWorkflow.RESULTS, {
    id: STORY_IDS.secondElection,
    presentation: electionPresentation("Deputy election"),
})
export const COUNCIL_CONTEST = contestRecord()
export const DEPUTY_CONTEST = contestRecord({
    id: STORY_IDS.secondContest,
    election_id: STORY_IDS.secondElection,
    max_votes: 1,
    winning_candidates_num: 1,
    presentation: {i18n: {en: {name: "Deputy", alias: "Deputy"}}},
})
export const DEPUTY_CANDIDATES = candidateRecords(STORY_IDS.secondContest).map(
    (candidate, index) => ({
        ...candidate,
        id: index ? TALLY_IDS.secondDeputyCandidate : TALLY_IDS.deputyCandidate,
        presentation: {i18n: {en: {name: index ? "Dan Example" : "Carol Example"}}},
    })
)

/** Participation and candidate counts of a results row; every total adds up. */
function counts(census: number, valid: number, invalid: number, blank = 0) {
    const total = valid + invalid
    return {
        elegible_census: census,
        total_votes: total,
        total_votes_percent: total / census,
        total_auditable_votes: total,
        total_auditable_votes_percent: 1,
        total_valid_votes: valid,
        total_valid_votes_percent: valid / total,
        total_invalid_votes: invalid,
        total_invalid_votes_percent: invalid / total,
        explicit_invalid_votes: invalid,
        explicit_invalid_votes_percent: invalid / total,
        implicit_invalid_votes: 0,
        implicit_invalid_votes_percent: 0,
        total_blank_votes: blank,
        total_blank_votes_percent: blank / total,
        explicit_blank_votes: blank,
        explicit_blank_votes_percent: blank / total,
        implicit_blank_votes: 0,
        implicit_blank_votes_percent: 0,
    }
}

/**
 * The council contest counts 84 valid votes: Alice 50 and Bob 34, of which the
 * north district cast 33 and 23 and the south district 17 and 11. The deputy
 * contest counts Carol 21 and Dan 17.
 */
export const CANDIDATE_VOTES = {
    council: {[STORY_IDS.candidate]: 50, [STORY_IDS.secondCandidate]: 34},
    north: {[STORY_IDS.candidate]: 33, [STORY_IDS.secondCandidate]: 23},
    south: {[STORY_IDS.candidate]: 17, [STORY_IDS.secondCandidate]: 11},
    deputy: {[TALLY_IDS.deputyCandidate]: 21, [TALLY_IDS.secondDeputyCandidate]: 17},
}

export interface TallyDataOptions {
    /** The council contest uses instant runoff, with its rounds in the annotations. */
    preferential?: boolean
    /** The council contest was decided by acclamation. */
    acclaimed?: boolean
    /** Results event the data was loaded for; another one keeps the widgets waiting. */
    resultsEventId?: string
}

const candidateRows = (
    votes: Record<string, number>,
    ids: {contest_id: string; election_id: string},
    prefix: string
) => {
    const valid = Object.values(votes).reduce((sum, count) => sum + count, 0)
    return Object.entries(votes).map(([candidate_id, cast_votes], index) => ({
        ...eventResults,
        ...ids,
        id: `${prefix}-${candidate_id}`,
        candidate_id,
        cast_votes,
        cast_votes_percent: cast_votes / valid,
        winning_position: index + 1,
        points: null,
        documents: null,
        annotations: null,
        labels: null,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }))
}

const runoffAnnotations = () =>
    text({
        process_results: {
            candidates_status: {
                [STORY_IDS.candidate]: "Active",
                [STORY_IDS.secondCandidate]: "Active",
            },
            name_references: [
                {id: STORY_IDS.candidate, name: "Alice Example"},
                {id: STORY_IDS.secondCandidate, name: "Bob Example"},
            ],
            round_count: 1,
            max_rounds: 1,
            rounds: [
                {
                    winner: {id: STORY_IDS.candidate, name: "Alice Example"},
                    candidates_wins: {
                        [STORY_IDS.candidate]: {
                            name: "Alice Example",
                            wins: 50,
                            transference: 0,
                            percentage: 50 / 84,
                        },
                        [STORY_IDS.secondCandidate]: {
                            name: "Bob Example",
                            wins: 34,
                            transference: 0,
                            percentage: 34 / 84,
                        },
                    },
                    eliminated_candidates: null,
                    active_candidates_count: 2,
                    active_ballots_count: 84,
                    exhausted_ballots_count: 0,
                },
            ],
        },
    })

/** What ResultsDataLoader stores in `tallyQueryData` once the tally has results. */
export function tallyData({
    preferential = false,
    acclaimed = false,
    resultsEventId = TALLY_IDS.resultsEvent,
}: TallyDataOptions = {}): GetTallyDataQuery {
    const council = {contest_id: STORY_IDS.contest, election_id: STORY_IDS.election}
    const deputy = {contest_id: STORY_IDS.secondContest, election_id: STORY_IDS.secondElection}
    const contestRow = (contest: typeof COUNCIL_CONTEST) => ({
        ...contest,
        counting_algorithm:
            preferential && contest.id === STORY_IDS.contest
                ? "instant-runoff"
                : contest.counting_algorithm,
        is_acclaimed: acclaimed && contest.id === STORY_IDS.contest,
        presentation: text(contest.presentation),
        annotations: text(contest.annotations),
        labels: text(contest.labels),
        is_active: true,
    })
    return {
        sequent_backend_election_event: [{presentation: text(eventPresentation)}],
        sequent_backend_area: [north, south].map((area) => ({
            ...area,
            annotations: text(area.annotations),
            labels: text(area.labels),
        })),
        sequent_backend_area_contest: [
            {id: STORY_IDS.areaContest, area_id: north.id, contest_id: STORY_IDS.contest},
            {id: storyId(8, 7), area_id: south.id, contest_id: STORY_IDS.contest},
            {id: storyId(8, 8), area_id: north.id, contest_id: STORY_IDS.secondContest},
        ].map((row) => ({...scope, ...row})),
        sequent_backend_election: [COUNCIL_ELECTION, DEPUTY_ELECTION].map((election) => ({
            ...election,
            presentation: text(election.presentation),
            status: text(election.status),
        })),
        sequent_backend_candidate: [
            ...candidateRecords(STORY_IDS.contest),
            ...DEPUTY_CANDIDATES,
        ].map((candidate) => ({...candidate, presentation: text(candidate.presentation)})),
        sequent_backend_contest: [COUNCIL_CONTEST, DEPUTY_CONTEST].map(contestRow),
        sequent_backend_results_event: [
            {
                ...scope,
                id: resultsEventId,
                name: "Council results",
                documents: text(RESULT_DOCUMENTS.event),
            },
        ],
        sequent_backend_results_election: [
            {
                ...eventResults,
                id: storyId(9, 3),
                election_id: STORY_IDS.election,
                elegible_census: 120,
                total_voters: 90,
                total_voters_percent: 0.75,
                documents: text(RESULT_DOCUMENTS.election),
            },
            {
                ...eventResults,
                id: storyId(9, 4),
                election_id: STORY_IDS.secondElection,
                elegible_census: 80,
                total_voters: 40,
                total_voters_percent: 0.5,
            },
        ],
        sequent_backend_results_contest: [
            {
                ...eventResults,
                ...council,
                ...counts(120, 84, 6, 3),
                id: storyId(9, 5),
                counting_algorithm: preferential ? "instant-runoff" : "plurality-at-large",
                annotations: preferential ? runoffAnnotations() : null,
                documents: RESULT_DOCUMENTS.contest,
            },
            {
                ...eventResults,
                ...deputy,
                ...counts(80, 38, 2),
                id: storyId(9, 6),
                counting_algorithm: "plurality-at-large",
            },
        ],
        sequent_backend_results_contest_candidate: [
            ...candidateRows(CANDIDATE_VOTES.council, council, "council"),
            ...candidateRows(CANDIDATE_VOTES.deputy, deputy, "deputy"),
        ],
        sequent_backend_results_area_contest: [
            {
                ...eventResults,
                ...council,
                ...counts(70, 56, 4),
                id: storyId(9, 7),
                area_id: north.id,
                annotations: text({extended_metrics: {weight: 2}}),
                documents: RESULT_DOCUMENTS.areaContest,
            },
            {
                ...eventResults,
                ...council,
                ...counts(50, 28, 2),
                id: storyId(9, 8),
                area_id: south.id,
            },
        ],
        sequent_backend_results_area_contest_candidate: [
            ...candidateRows(CANDIDATE_VOTES.north, council, "north").map((row) => ({
                ...row,
                area_id: north.id,
            })),
            ...candidateRows(CANDIDATE_VOTES.south, council, "south").map((row) => ({
                ...row,
                area_id: south.id,
            })),
        ],
        sequent_backend_results_election_area: [
            {
                ...eventResults,
                id: storyId(9, 9),
                election_id: STORY_IDS.election,
                area_id: north.id,
                name: "North district",
                documents: text(RESULT_DOCUMENTS.electionArea),
                created_at: FIXED_TIME,
                last_updated_at: FIXED_TIME,
            },
        ],
    }
}

/** The tally's trustees and elections as its execution reports them at each step. */
export function executionStatus(workflow: EStoryWorkflow): ITallyCeremonyStatus {
    const completed = workflow === EStoryWorkflow.RESULTS
    return {
        trustees: trusteeRecords.slice(0, 2).map(({name}) => ({
            name: String(name),
            status: ITallyTrusteeStatus.KEY_RESTORED,
        })),
        elections_status: [
            {
                election_id: STORY_IDS.election,
                status: completed ? ITallyElectionStatus.SUCCESS : ITallyElectionStatus.MIXING,
                progress: completed ? 100 : 40,
            },
            {
                election_id: STORY_IDS.secondElection,
                status: completed ? ITallyElectionStatus.SUCCESS : ITallyElectionStatus.WAITING,
                progress: completed ? 100 : 0,
            },
        ],
        logs: [
            {created_date: "2026-01-15T12:00:00Z", log_text: "Tally session created"},
            ...(completed
                ? [{created_date: "2026-01-15T12:30:00Z", log_text: "Tally completed"}]
                : []),
        ],
    }
}

/**
 * The event's tally over both elections: its trustees are connected while the
 * ceremony runs and it has completed once results are published.
 */
export const tallySession = (
    workflow: EStoryWorkflow,
    overrides: Partial<StoryRecord<Sequent_Backend_Tally_Session>> = {}
) =>
    tallySessionRecord(workflow, {
        election_ids: [STORY_IDS.election, STORY_IDS.secondElection],
        area_ids: [north.id, south.id],
        ...overrides,
    }) as Sequent_Backend_Tally_Session

export function tallyExecution(
    workflow: EStoryWorkflow,
    overrides: Partial<StoryRecord<Sequent_Backend_Tally_Session_Execution>> = {}
): Sequent_Backend_Tally_Session_Execution {
    return {
        ...scope,
        id: TALLY_IDS.execution,
        tally_session_id: STORY_IDS.tallySession,
        current_message_id: 12,
        session_ids: [1, 2],
        results_event_id: workflow === EStoryWorkflow.RESULTS ? TALLY_IDS.resultsEvent : null,
        status: executionStatus(workflow),
        documents: workflow === EStoryWorkflow.RESULTS ? {sqlite: documentId(90)} : null,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
        ...overrides,
    }
}

export const resultsEvent = (
    documents: IResultDocuments = RESULT_DOCUMENTS.event
): Sequent_Backend_Results_Event => ({
    ...scope,
    id: TALLY_IDS.resultsEvent,
    name: "Council results",
    documents,
    created_at: FIXED_TIME,
    last_updated_at: FIXED_TIME,
})

export interface TallyContextValues {
    tallyId?: string | null
    selectedTallySessionData?: IMiruTransmissionPackageData | null
    /** The loaded results; `null` while ResultsDataLoader has not loaded them. */
    data?: GetTallyDataQuery | null
    /** Observes the tally screen's selection of another tally, e.g. going back. */
    onSetTallyId?: (tallyId: string | null) => void
    onSetMiruAreaId?: (areaId: string) => void
}

/**
 * The tally screen's shared state as TallyCeremony's parents provide it: the
 * selected tally and transmission package, and the loaded results.
 */
export function TallyStoryContext({
    tallyId = STORY_IDS.tallySession,
    selectedTallySessionData = null,
    data = null,
    onSetTallyId,
    onSetMiruAreaId,
    children,
}: PropsWithChildren<TallyContextValues>) {
    const context = useContext(ElectionEventTallyContext)
    const [store] = useState(() => {
        const atoms = createStore()
        atoms.set(tallyQueryData, data)
        return atoms
    })
    const [selectedTally, setSelectedTally] = useState(tallyId)
    const [selectedPackage, setSelectedPackage] = useState(selectedTallySessionData)
    const [miruAreaId, setMiruAreaId] = useState<string | null>(
        selectedTallySessionData?.area_id ?? null
    )
    return (
        <AtomProvider store={store}>
            <ElectionEventTallyContext.Provider
                value={{
                    ...context,
                    tallyId: selectedTally,
                    setTallyId: (id) => {
                        onSetTallyId?.(id)
                        setSelectedTally(id)
                    },
                    electionEventId: EVENT_ID,
                    selectedTallySessionData: selectedPackage,
                    setSelectedTallySessionData: setSelectedPackage,
                    miruAreaId,
                    setMiruAreaId: (id) => {
                        onSetMiruAreaId?.(id)
                        setMiruAreaId(id)
                    },
                }}
            >
                {children}
            </ElectionEventTallyContext.Provider>
        </AtomProvider>
    )
}

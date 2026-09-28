// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Small typed synthetic records shared by the admin widget stories. Every
// value is invented; the IDs follow the pattern of the existing screen stories.
import {FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import {
    EConsolidatedReportPolicy,
    EElectionEventDelegatedVotingPolicy,
    EVotingStatus,
    type IElectionEventPresentation,
    type IElectionEventStatus,
    type IElectionPresentation,
    type IElectionStatus,
} from "@sequentech/ui-core"
import type {
    Sequent_Backend_Area,
    Sequent_Backend_Area_Contest,
    Sequent_Backend_Candidate,
    Sequent_Backend_Contest,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Keys_Ceremony,
    Sequent_Backend_Tally_Session,
    Sequent_Backend_Tenant,
    Sequent_Backend_Trustee,
} from "@/gql/graphql"
import {IKeysCeremonyExecutionStatus, IKeysCeremonyTrusteeStatus} from "@/services/KeyCeremony"
import {ETallyType, ITallyExecutionStatus} from "@/types/ceremonies"
import {EStoryWorkflow} from "../../../ui-essentials/.storybook/globals"
import {EVENT_ID, TENANT_ID} from "./AdminStoryProvider"
import {STORY_TRUSTEE} from "./storyAuth"

export {FIXED_TIME}

type IsAny<T> = 0 extends 1 & T ? true : false
type IsRelation<V> =
    NonNullable<V> extends ReadonlyArray<infer E>
        ? IsAny<E> extends true
            ? false
            : E extends {__typename?: string}
              ? true
              : false
        : NonNullable<V> extends {nodes: unknown}
          ? true
          : false

/**
 * A row of a generated table type without its relationship fields, which
 * Hasura only returns when a query selects them.
 */
export type StoryRecord<T> = {
    [K in keyof T as K extends "__typename"
        ? never
        : IsRelation<T[K]> extends true
          ? never
          : K]: T[K]
}

/** Stable IDs; a story that needs another record of a kind derives it with `storyId`. */
export const STORY_IDS = {
    tenant: TENANT_ID,
    event: EVENT_ID,
    election: "33333333-3333-4333-8333-333333333333",
    secondElection: "33333333-3333-4333-8333-333333333334",
    contest: "44444444-4444-4444-8444-444444444441",
    secondContest: "44444444-4444-4444-8444-444444444442",
    candidate: "66666666-6666-4666-8666-666666666661",
    secondCandidate: "66666666-6666-4666-8666-666666666662",
    area: "77777777-7777-4777-8777-777777777771",
    secondArea: "77777777-7777-4777-8777-777777777772",
    areaContest: "78787878-7878-4787-8787-787878787871",
    keysCeremony: "44444444-4444-4444-8444-444444444444",
    tallySession: "55555555-5555-4555-8555-555555555555",
    user: "88888888-8888-4888-8888-888888888881",
    secondUser: "88888888-8888-4888-8888-888888888882",
} as const

/** A further UUID of the same shape, e.g. `storyId(9, 3)` for a third record of kind 9. */
export const storyId = (kind: number, index: number) => {
    const digit = String(kind % 10)
    const block = (length: number) => digit.repeat(length)
    return `${block(8)}-${block(4)}-4${block(3)}-8${block(3)}-${block(11)}${index % 10}`
}

export const LANGUAGE_CONF = {enabled_language_codes: ["en", "es"], default_language_code: "en"}

export const tenantRecord: StoryRecord<Sequent_Backend_Tenant> = {
    id: TENANT_ID,
    slug: "example-council",
    is_active: true,
    annotations: {},
    labels: {},
    settings: {language_conf: LANGUAGE_CONF, languages: ["en", "es"]},
    voting_channels: {online: true, kiosk: true, early_voting: false, telephone: false},
    created_at: FIXED_TIME,
    updated_at: FIXED_TIME,
    test: 1,
}

const votingStatus = (workflow: EStoryWorkflow): EVotingStatus =>
    (
        ({
            [EStoryWorkflow.STARTED]: EVotingStatus.OPEN,
            [EStoryWorkflow.ENDED]: EVotingStatus.CLOSED,
            [EStoryWorkflow.TALLY]: EVotingStatus.CLOSED,
            [EStoryWorkflow.RESULTS]: EVotingStatus.CLOSED,
        }) as Partial<Record<EStoryWorkflow, EVotingStatus>>
    )[workflow] ?? EVotingStatus.NOT_STARTED

const isPublished = (workflow: EStoryWorkflow) =>
    ![EStoryWorkflow.CREATED, EStoryWorkflow.KEYS].includes(workflow)

/** Voting period dates the election records after each step. */
function periodDates(workflow: EStoryWorkflow): IElectionStatus["voting_period_dates"] {
    const status = votingStatus(workflow)
    if (status === EVotingStatus.NOT_STARTED) return {}
    const started = {
        first_started_at: "2026-01-10T08:00:00Z",
        last_started_at: "2026-01-10T08:00:00Z",
    }
    return status === EVotingStatus.OPEN
        ? started
        : {
              ...started,
              first_stopped_at: "2026-01-14T20:00:00Z",
              last_stopped_at: "2026-01-14T20:00:00Z",
          }
}

/** The event status at a workflow step: online voting follows the step, other channels stay closed. */
export function eventStatus(workflow: EStoryWorkflow): IElectionEventStatus {
    return {
        is_published: isPublished(workflow),
        voting_status: votingStatus(workflow),
        kiosk_voting_status: EVotingStatus.NOT_STARTED,
        early_voting_status: EVotingStatus.NOT_STARTED,
        telephone_voting_status: EVotingStatus.NOT_STARTED,
    }
}

export function electionStatus(workflow: EStoryWorkflow): IElectionStatus {
    return {
        ...eventStatus(workflow),
        voting_period_dates: periodDates(workflow),
        kiosk_voting_period_dates: {},
        early_voting_period_dates: {},
        telephone_voting_period_dates: {},
    }
}

export const eventPresentation: IElectionEventPresentation = {
    i18n: {
        en: {name: "Council event", alias: "Council", description: "Synthetic event"},
        es: {name: "Evento del consejo", alias: "Consejo", description: "Evento sintético"},
    },
    language_conf: LANGUAGE_CONF,
    delegated_voting_policy: EElectionEventDelegatedVotingPolicy.DISABLED,
}

export function eventRecord(
    workflow: EStoryWorkflow = EStoryWorkflow.ENDED,
    overrides: Partial<StoryRecord<Sequent_Backend_Election_Event>> = {}
): StoryRecord<Sequent_Backend_Election_Event> {
    return {
        id: EVENT_ID,
        tenant_id: TENANT_ID,
        description: "Synthetic election event for stories",
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
        is_archived: false,
        is_audit: false,
        annotations: {},
        labels: {},
        presentation: eventPresentation,
        status: eventStatus(workflow),
        voting_channels: {online: true, kiosk: false, early_voting: false, telephone: false},
        statistics: {num_emails_sent: 12, num_sms_sent: 3},
        bulletin_board_reference: {},
        encryption_protocol: "RSA256",
        public_key:
            isPublished(workflow) || workflow === EStoryWorkflow.KEYS ? "c3ludGhldGlj" : null,
        ...overrides,
    }
}

export const electionPresentation = (
    name: string,
    alias = name.split(" ")[0]
): IElectionPresentation => ({
    i18n: {en: {name, alias}, es: {name: `${name} (es)`, alias}},
    language_conf: LANGUAGE_CONF,
    consolidated_report_policy: EConsolidatedReportPolicy.DO_NOT_GENERATE,
})

export function electionRecord(
    workflow: EStoryWorkflow = EStoryWorkflow.ENDED,
    overrides: Partial<StoryRecord<Sequent_Backend_Election>> = {}
): StoryRecord<Sequent_Backend_Election> {
    return {
        id: STORY_IDS.election,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        description: "Choose the council members",
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
        presentation: electionPresentation("Council election"),
        status: electionStatus(workflow),
        voting_channels: {online: true, kiosk: false, early_voting: false, telephone: false},
        keys_ceremony_id: STORY_IDS.keysCeremony,
        num_allowed_revotes: 1,
        spoil_ballot_option: false,
        is_consolidated_ballot_encoding: false,
        is_kiosk: false,
        permission_label: null,
        ...overrides,
    }
}

export function contestRecord(
    overrides: Partial<StoryRecord<Sequent_Backend_Contest>> = {}
): StoryRecord<Sequent_Backend_Contest> {
    return {
        id: STORY_IDS.contest,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        election_id: STORY_IDS.election,
        description: "Select up to two members",
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
        min_votes: 0,
        max_votes: 2,
        winning_candidates_num: 2,
        voting_type: "non-preferential",
        counting_algorithm: "plurality-at-large",
        is_encrypted: true,
        is_acclaimed: false,
        presentation: {
            i18n: {
                en: {name: "Council members", alias: "Members"},
                es: {name: "Miembros del consejo", alias: "Miembros"},
            },
        },
        ...overrides,
    }
}

export function candidateRecords(
    contestId: string = STORY_IDS.contest
): StoryRecord<Sequent_Backend_Candidate>[] {
    return [
        {id: STORY_IDS.candidate, name: "Alice Example", sort_order: 0},
        {id: STORY_IDS.secondCandidate, name: "Bob Example", sort_order: 1},
    ].map(({id, name, sort_order}) => ({
        id,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        contest_id: contestId,
        description: `${name} stands for the council`,
        type: "candidate",
        is_public: true,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
        presentation: {i18n: {en: {name, alias: name.split(" ")[0]}}, sort_order},
    }))
}

export function areaRecords(): StoryRecord<Sequent_Backend_Area>[] {
    return [
        {id: STORY_IDS.area, name: "North district", type: "district"},
        {id: STORY_IDS.secondArea, name: "South district", type: "district"},
    ].map(({id, name, type}) => ({
        id,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        name,
        description: `${name} voters`,
        type,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
        presentation: {},
        parent_id: null,
    }))
}

export function areaContestRecord(
    overrides: Partial<StoryRecord<Sequent_Backend_Area_Contest>> = {}
): StoryRecord<Sequent_Backend_Area_Contest> {
    return {
        id: STORY_IDS.areaContest,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        area_id: STORY_IDS.area,
        contest_id: STORY_IDS.contest,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
        ...overrides,
    }
}

export const trusteeRecords: StoryRecord<Sequent_Backend_Trustee>[] = [
    STORY_TRUSTEE,
    "trustee2",
    "trustee3",
].map((name, index) => ({
    id: `55555555-5555-4555-8555-55555555555${index}`,
    tenant_id: TENANT_ID,
    name,
    public_key: `synthetic-public-key-${index}`,
    created_at: FIXED_TIME,
    last_updated_at: FIXED_TIME,
    annotations: {},
    labels: {},
}))

/** The keys ceremony runs while the event is created and has completed afterwards. */
export function keysCeremonyRecord(
    workflow: EStoryWorkflow = EStoryWorkflow.ENDED,
    overrides: Partial<StoryRecord<Sequent_Backend_Keys_Ceremony>> = {}
): StoryRecord<Sequent_Backend_Keys_Ceremony> {
    const generating = workflow === EStoryWorkflow.CREATED
    return {
        id: STORY_IDS.keysCeremony,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        name: "Council key",
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        trustee_ids: trusteeRecords.map(({id}) => id),
        threshold: 2,
        execution_status: generating
            ? IKeysCeremonyExecutionStatus.IN_PROGRESS
            : IKeysCeremonyExecutionStatus.SUCCESS,
        status: {
            public_key: generating ? undefined : "synthetic-public-key",
            trustees: trusteeRecords.map(({name}, index) => ({
                name,
                status:
                    generating && index !== 1
                        ? IKeysCeremonyTrusteeStatus.WAITING
                        : IKeysCeremonyTrusteeStatus.KEY_CHECKED,
            })),
            logs: [],
        },
        settings: {policy: "manual-ceremonies"},
        is_default: true,
        annotations: {},
        labels: {},
        ...overrides,
    }
}

/** The tally session exists from the tally step: connected trustees, then completed. */
export function tallySessionRecord(
    workflow: EStoryWorkflow = EStoryWorkflow.TALLY,
    overrides: Partial<StoryRecord<Sequent_Backend_Tally_Session>> = {}
): StoryRecord<Sequent_Backend_Tally_Session> {
    const completed = workflow === EStoryWorkflow.RESULTS
    return {
        id: STORY_IDS.tallySession,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        keys_ceremony_id: STORY_IDS.keysCeremony,
        election_ids: [STORY_IDS.election],
        area_ids: [STORY_IDS.area],
        execution_status: completed
            ? ITallyExecutionStatus.SUCCESS
            : ITallyExecutionStatus.CONNECTED,
        is_execution_completed: completed,
        threshold: 2,
        tally_type: ETallyType.ELECTORAL_RESULTS,
        configuration: {},
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
        ...overrides,
    }
}

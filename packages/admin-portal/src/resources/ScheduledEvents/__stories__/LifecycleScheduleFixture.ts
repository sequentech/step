// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The Scheduled Events screens under the two timezone configurations: the
// event, its elections and schedule as records, with the instants computed
// from each row's wall time and zone, and the predicted outcomes.
import {
    EScheduledOutcomeKind,
    zonedToInstant,
    type IElectionEventPresentation,
    type IScheduledOutcomeExplanation,
} from "@sequentech/ui-core"
import type {
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Scheduled_Event,
} from "@/gql/graphql"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {
    FIXED_TIME,
    electionPresentation,
    electionRecord,
    eventPresentation,
    eventRecord,
    type StoryRecord,
} from "@/__stories__/fixtures"
import type {ITimeZoneConfiguration} from "@/components/timezones/__fixtures__/configurations"
import type {ILifecycleSnapshotEntry} from "@/queries/Lifecycle"
import type {IScheduledOutcomeRow, IScheduledTransition} from "@/types/lifecycle"

export const EVENT_RESOURCE = "sequent_backend_election_event"

export const lifecycleEvent = (
    configuration: ITimeZoneConfiguration
): StoryRecord<Sequent_Backend_Election_Event> =>
    eventRecord(undefined, {
        presentation: {
            ...eventPresentation,
            i18n: {en: {name: configuration.eventName, alias: configuration.eventName}},
            ...configuration.presentation,
        } as IElectionEventPresentation,
    })

export const lifecycleElections = (
    configuration: ITimeZoneConfiguration
): Array<StoryRecord<Sequent_Backend_Election>> =>
    configuration.elections.map(({id, name, timezone}) =>
        electionRecord(undefined, {
            id,
            presentation: {...electionPresentation(name, name), timezone},
        })
    )

/** The instant of a row's wall time in its zone. */
export const instantOf = (local: string, zone: string) => zonedToInstant(local, zone).instant

export const lifecycleSchedule = (
    configuration: ITimeZoneConfiguration
): Array<StoryRecord<Sequent_Backend_Scheduled_Event>> =>
    configuration.schedule.map((row) => ({
        id: row.id,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        event_processor: row.event_processor,
        cron_config: {
            scheduled_date: instantOf(row.local, row.timezone),
            local: row.local,
            timezone: row.timezone,
        },
        event_payload: {election_id: row.election_id},
        created_at: FIXED_TIME,
        created_by: "admin",
        archived_at: null,
        stopped_at: row.stopped_at ?? null,
        task_id: null,
        annotations: {},
        labels: {},
    }))

const VOTING = ["START_VOTING_PERIOD", "END_VOTING_PERIOD"]

/**
 * The predicted outcome of each future opening and closing, one per row and
 * election (event-wide rows have one per election): `outcomeOf` picks it.
 */
export const lifecycleOutcomes = (
    configuration: ITimeZoneConfiguration,
    outcomeOf: (row: {
        id: string
        event_processor: string
        election_id: string
    }) => IScheduledOutcomeExplanation
): Array<IScheduledOutcomeRow> =>
    configuration.schedule
        .filter((row) => VOTING.includes(row.event_processor) && !row.stopped_at)
        .flatMap((row) =>
            (row.election_id ? [row.election_id] : configuration.elections.map(({id}) => id)).map(
                (election_id) => ({
                    scheduled_event_id: row.id,
                    election_id,
                    explanation: outcomeOf({...row, election_id}),
                })
            )
        )

/** The event's signed snapshot (`get_lifecycle_snapshots`), listing the schedule as it is now. */
export const lifecycleSnapshotEntry = (
    configuration: ITimeZoneConfiguration,
    publishedAt = "2028-03-01T00:00:00Z"
): ILifecycleSnapshotEntry => ({
    election_id: null,
    publication_id: "99999999-9999-4999-8999-999999999990",
    published_at: publishedAt,
    approval_request_id: "99999999-9999-4999-8999-999999999991",
    approval_code: "K7Q-2M",
    signed: true,
    snapshot: {
        policies: configuration.presentation.lifecycle_policies ?? null,
        open_voting: {required: true, signatures: 2},
        close_voting: {required: true, signatures: 2},
        schedule: configuration.schedule
            .filter((row) => VOTING.includes(row.event_processor))
            .map<IScheduledTransition>((row) => ({
                scheduled_event_id: row.id,
                event_processor: row.event_processor,
                election_id: row.election_id,
                scheduled_date: instantOf(row.local, row.timezone),
                local: row.local,
                timezone: row.timezone,
                voting_channels: null,
                fingerprint: `fingerprint-${row.id}`,
            })),
    },
})

export const REFUSED = EScheduledOutcomeKind.REFUSED

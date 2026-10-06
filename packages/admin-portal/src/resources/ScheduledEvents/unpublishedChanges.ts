// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Voters see schedule changes only after publication (design §8): which rows
// changed since the publication that covers them.
import type {Sequent_Backend_Scheduled_Event} from "@/gql/graphql"
import {newestPerTarget, type ILifecycleSnapshotEntry} from "@/queries/Lifecycle"
import type {ICronConfig, IManageElectionDatePayload} from "@/types/scheduledEvents"
import type {IScheduledTransition} from "@/types/lifecycle"

type ScheduledEvent = Pick<
    Sequent_Backend_Scheduled_Event,
    "id" | "event_processor" | "cron_config" | "event_payload" | "created_at" | "stopped_at"
>

/** The newest publication (of `publications`, newest first) that covers an election: its own, or the event's. */
export const publicationFor = (
    publications: ReadonlyArray<ILifecycleSnapshotEntry>,
    electionId: string
): ILifecycleSnapshotEntry | null =>
    publications.find(
        (publication) => !publication.election_id || publication.election_id === electionId
    ) ?? null

const sameInstant = (a: string | null | undefined, b: string | null | undefined) =>
    (a ? Date.parse(a) : NaN) === (b ? Date.parse(b) : NaN) || (!a && !b)

const sameChannels = (a: Array<string> | null | undefined, b: Array<string> | null | undefined) =>
    [...(a ?? [])].sort().join("|") === [...(b ?? [])].sort().join("|")

/** Whether a row differs from what a publication recorded of it. */
const differs = (event: ScheduledEvent, published: IScheduledTransition | undefined) => {
    if (!published) return true
    const cron = (event.cron_config ?? {}) as ICronConfig
    const payload = (event.event_payload ?? {}) as IManageElectionDatePayload
    return (
        !sameInstant(cron.scheduled_date, published.scheduled_date) ||
        (cron.local ?? null) !== (published.local ?? null) ||
        (cron.timezone ?? null) !== (published.timezone ?? null) ||
        !sameChannels(payload.voting_channels, published.voting_channels)
    )
}

/** Whether voters of an election see this row as it is (after `publication`). */
const seenAsIs = (event: ScheduledEvent, publication: ILifecycleSnapshotEntry | null): boolean => {
    if (!publication) return false
    const schedule = publication.snapshot?.schedule
    const voting =
        event.event_processor === "START_VOTING_PERIOD" ||
        event.event_processor === "END_VOTING_PERIOD"
    if (voting && schedule) {
        return !differs(
            event,
            schedule.find(({scheduled_event_id}) => scheduled_event_id === event.id)
        )
    }
    return Date.parse(event.created_at) <= Date.parse(publication.published_at)
}

/**
 * The ids of rows voters don't see yet. A publication's lifecycle snapshot
 * lists its openings and closings, so those are compared field by field;
 * other rows count as changed when created after the publication. An
 * event-wide row is seen only when every election's covering publication has
 * it as it is. Rows that already ran, and every row before the first
 * publication, aren't marked.
 */
export const unpublishedEventIds = (
    events: ReadonlyArray<ScheduledEvent>,
    publications: ReadonlyArray<ILifecycleSnapshotEntry>,
    electionIds: ReadonlyArray<string> = []
): Set<string> => {
    const ids = new Set<string>()
    if (!publications.length) return ids
    const newest = newestPerTarget(publications)
    for (const event of events) {
        if (event.stopped_at) continue
        const electionId = (event.event_payload as IManageElectionDatePayload | null)?.election_id
        const targets = electionId ? [electionId] : electionIds.length ? electionIds : [null]
        const seen = targets.every((target) =>
            seenAsIs(event, target ? publicationFor(newest, target) : (newest[0] ?? null))
        )
        if (!seen) ids.add(String(event.id))
    }
    return ids
}

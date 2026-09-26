// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic scheduled events of the election event and the elections they target.
import {VotingStatusChannel} from "@sequentech/ui-core"
import type {Sequent_Backend_Election, Sequent_Backend_Scheduled_Event} from "@/gql/graphql"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {
    FIXED_TIME,
    STORY_IDS,
    electionPresentation,
    electionRecord,
    storyId,
    type StoryRecord,
} from "@/__stories__/fixtures"
import {EventProcessors} from "../CreateScheduledEvent"

export const SCHEDULED_EVENT_RESOURCE = "sequent_backend_scheduled_event"
export const ELECTION_RESOURCE = "sequent_backend_election"
export const VOTING_START_ID = storyId(0, 1)
export const TALLY_ID = storyId(0, 2)
export const VOTING_START_DATE = "2026-11-02T08:00:00.000Z"
export const TALLY_DATE = "2026-11-03T20:00:00.000Z"

export const scheduledElections = (): StoryRecord<Sequent_Backend_Election>[] => [
    electionRecord(),
    electionRecord(undefined, {
        id: STORY_IDS.secondElection,
        external_id: "DEP-2026",
        presentation: electionPresentation("Deputy election"),
    }),
]

const scheduledEvent = (
    id: string,
    processor: EventProcessors,
    scheduledDate: string,
    payload: Record<string, unknown>
): StoryRecord<Sequent_Backend_Scheduled_Event> => ({
    id,
    tenant_id: TENANT_ID,
    election_event_id: EVENT_ID,
    event_processor: processor,
    cron_config: {scheduled_date: scheduledDate},
    event_payload: payload,
    created_at: FIXED_TIME,
    created_by: "admin",
    archived_at: null,
    stopped_at: null,
    task_id: null,
    annotations: {},
    labels: {},
})

export const scheduledEventRecords = (): StoryRecord<Sequent_Backend_Scheduled_Event>[] => [
    scheduledEvent(VOTING_START_ID, EventProcessors.START_VOTING_PERIOD, VOTING_START_DATE, {
        election_id: STORY_IDS.election,
        voting_channels: [VotingStatusChannel.Online],
    }),
    scheduledEvent(TALLY_ID, EventProcessors.ALLOW_TALLY, TALLY_DATE, {
        election_id: STORY_IDS.secondElection,
    }),
]

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic notifications of the election event.
import type {Sequent_Backend_Notification} from "@/gql/graphql"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME, STORY_IDS, storyId, type StoryRecord} from "@/__stories__/fixtures"

export const NOTIFICATION_RESOURCE = "sequent_backend_notification"

export const notificationRecords = (): StoryRecord<Sequent_Backend_Notification>[] => [
    {
        id: storyId(8, 1),
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        election_id: STORY_IDS.election,
        name: "Polls opening reminder",
        alias: "polls-open",
        type: "ALL_ELECTIONS_OPEN",
        template_id: null,
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
    },
]

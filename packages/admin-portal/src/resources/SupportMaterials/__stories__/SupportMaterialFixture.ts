// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic support materials of the election event.
import type {
    Sequent_Backend_Document,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Support_Material,
} from "@/gql/graphql"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {FIXED_TIME, eventRecord, storyId, type StoryRecord} from "@/__stories__/fixtures"

export const MATERIAL_RESOURCE = "sequent_backend_support_material"
export const EVENT_RESOURCE = "sequent_backend_election_event"
export const DOCUMENT_RESOURCE = "sequent_backend_document"
export const MATERIAL_ID = storyId(9, 1)
export const MATERIAL_DOCUMENT_ID = storyId(9, 2)
export const UPLOADED_DOCUMENT_ID = storyId(9, 3)
export const UPLOAD_URL = "https://s3.admin-story.invalid/upload/support-material"

/** The event as the event form's record carries it, with its languages as switches. */
export const materialEvent = (): StoryRecord<Sequent_Backend_Election_Event> & {
    enabled_languages: Record<string, boolean>
} => ({...eventRecord(), enabled_languages: {en: true, es: true, fr: false}})

export const materialRecords = (): StoryRecord<Sequent_Backend_Support_Material>[] => [
    {
        id: MATERIAL_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        data: {
            title_i18n: {en: "Voting guide", es: "Guía de voto"},
            subtitle_i18n: {en: "How to vote online", es: "Cómo votar en línea"},
        },
        kind: "application/pdf",
        document_id: MATERIAL_DOCUMENT_ID,
        is_hidden: false,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
    },
]

export const documentRecords = (): StoryRecord<Sequent_Backend_Document>[] => [
    {
        id: MATERIAL_DOCUMENT_ID,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        name: "voting-guide.pdf",
        media_type: "application/pdf",
        size: 2048,
        is_public: true,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
    },
]

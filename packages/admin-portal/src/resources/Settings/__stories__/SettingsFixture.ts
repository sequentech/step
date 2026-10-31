// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic records of the tenant settings tabs.
import type {FC} from "react"
import type {
    Sequent_Backend_Election_Type,
    Sequent_Backend_Preview,
    Sequent_Backend_Tenant,
    Sequent_Backend_Trustee,
} from "@/gql/graphql"
import {
    FIXED_TIME,
    LANGUAGE_CONF,
    storyId,
    tenantRecord,
    type StoryRecord,
} from "@/__stories__/fixtures"
import {TENANT_ID} from "@/__stories__/AdminStoryProvider"

export const TENANT_RESOURCE = "sequent_backend_tenant"
export const ELECTION_TYPE_RESOURCE = "sequent_backend_election_type"
export const TRUSTEE_RESOURCE = "sequent_backend_trustee"
export const PREVIEW_RESOURCE = "sequent_backend_preview"

export const GENERAL_TYPE_ID = storyId(3, 1)
export const REFERENDUM_TYPE_ID = storyId(3, 2)
export const PREVIEW_DOCUMENT_ID = storyId(5, 7)
export const FIRST_TRUSTEE_ID = storyId(4, 1)
export const SECOND_TRUSTEE_ID = storyId(4, 2)

/** The tenant with the settings each tab reads. */
export function settingsTenant(
    settings: Record<string, unknown> = {},
    overrides: Partial<StoryRecord<Sequent_Backend_Tenant>> = {}
): StoryRecord<Sequent_Backend_Tenant> {
    return {
        ...tenantRecord,
        annotations: {logo_url: "https://assets.admin-story.invalid/logo.svg", css: ""},
        settings: {
            language_conf: LANGUAGE_CONF,
            voting_countries: ["ES"],
            enroll_countries: [],
            gapi_email: "calendar@admin-story.invalid",
            help_links: [],
            i18n: {
                en: {"adminPortal:header.welcome": "Welcome, council"},
            },
            ...settings,
        },
        ...overrides,
    }
}

export const electionTypeRecords = (): StoryRecord<Sequent_Backend_Election_Type>[] =>
    [
        {id: GENERAL_TYPE_ID, name: "General election"},
        {id: REFERENDUM_TYPE_ID, name: "Referendum"},
    ].map(({id, name}) => ({
        id,
        name,
        tenant_id: TENANT_ID,
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
    }))

export const trusteeRecords = (): StoryRecord<Sequent_Backend_Trustee>[] =>
    [
        {id: FIRST_TRUSTEE_ID, name: "trustee1", public_key: "Q2VydGlmaWVkIGtleSBvbmU"},
        {id: SECOND_TRUSTEE_ID, name: "trustee2", public_key: "Q2VydGlmaWVkIGtleSB0d28"},
    ].map((trustee) => ({
        ...trustee,
        tenant_id: TENANT_ID,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        annotations: {},
        labels: {},
    }))

export const previewRecords = (): StoryRecord<Sequent_Backend_Preview>[] => [
    {
        id: storyId(5, 1),
        tenant_id: TENANT_ID,
        requested_by: "admin",
        url: "https://voting.admin-story.invalid/preview/council",
        document_id: PREVIEW_DOCUMENT_ID,
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
        annotations: {},
    },
]

/** The settings tabs are typed `FC<void>`; React renders them with empty props. */
export const settingsTab = (tab: FC<void>) => tab as unknown as FC

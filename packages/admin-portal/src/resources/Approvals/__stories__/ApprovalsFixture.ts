// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic voter applications and the user profile their approval screens read.
import type {Sequent_Backend_Applications, UserProfileAttribute} from "@/gql/graphql"
import {FIXED_TIME, STORY_IDS, storyId, type StoryRecord} from "@/__stories__/fixtures"
import {IApplicationsStatus} from "@/types/applications"

export const APPLICATION_ID = storyId(9, 1)
export const SECOND_APPLICATION_ID = storyId(9, 2)

export function applicationRecord(
    overrides: Partial<StoryRecord<Sequent_Backend_Applications>> = {}
): StoryRecord<Sequent_Backend_Applications> {
    return {
        id: APPLICATION_ID,
        tenant_id: STORY_IDS.tenant,
        election_event_id: STORY_IDS.event,
        area_id: STORY_IDS.area,
        applicant_id: "applicant-0001",
        applicant_data: {
            firstName: "Alice",
            lastName: "Example",
            email: "alice@example.test",
            dateOfBirth: "1990-05-17",
            embassy: "Madrid",
        },
        annotations: {"search-attributes": "firstName,lastName"},
        labels: {},
        permission_label: null,
        status: IApplicationsStatus.PENDING,
        verification_type: "MANUAL",
        created_at: FIXED_TIME,
        updated_at: FIXED_TIME,
        ...overrides,
    }
}

const attribute = (
    name: string,
    display_name: string,
    extra: Partial<UserProfileAttribute> = {}
): UserProfileAttribute => ({
    name,
    display_name,
    annotations: {},
    group: null,
    multivalued: false,
    permissions: null,
    required: null,
    selector: null,
    validations: {},
    ...extra,
})

/** `get_user_profile_attributes` of the event realm. */
export const APPROVAL_ATTRIBUTES: UserProfileAttribute[] = [
    attribute("first_name", "${firstName}"),
    attribute("last_name", "${lastName}"),
    attribute("email", "${email}"),
    attribute("dateOfBirth", "Date of birth", {annotations: {inputType: "html5-date"}}),
    attribute("embassy", "Embassy"),
]

/** Registered voters the approval screen offers as matches. */
export const MATCHING_VOTERS = [
    {
        id: STORY_IDS.user,
        username: "alice.example",
        first_name: "Alice",
        last_name: "Example",
        email: "alice@example.test",
        enabled: true,
        email_verified: true,
        attributes: {dateOfBirth: ["1990-05-17"], embassy: ["Madrid"]},
    },
    {
        id: STORY_IDS.secondUser,
        username: "alicia.example",
        first_name: "Alicia",
        last_name: "Example",
        email: "alicia@example.test",
        enabled: true,
        email_verified: false,
        attributes: {dateOfBirth: ["1991-06-18"], embassy: ["Lisbon"]},
    },
]

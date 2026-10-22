// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic enrollments, the voter registry they are compared with and the
// user profile their approval screens read.
import type {Sequent_Backend_Applications, UserProfileAttribute} from "@/gql/graphql"
import {STORY_IDS, storyId, type StoryRecord} from "@/__stories__/fixtures"
import {IApplicationsStatus} from "@/types/applications"

/** Alice: verified by ID scan, her date of birth differs from the registry. Needs review. */
export const APPLICATION_ID = storyId(9, 1)
/** Bob: an officer approved him. */
export const SECOND_APPLICATION_ID = storyId(9, 2)
/** Carol: typed her details by hand. Needs review and a face-to-face check. */
export const MANUAL_APPLICATION_ID = storyId(9, 3)
/** Dan: the rules rejected him, no voter in the registry has his details. */
export const REJECTED_APPLICATION_ID = storyId(9, 4)

export const CAROL_VOTER_ID = storyId(8, 3)
export const BOB_VOTER_ID = storyId(8, 4)

const COMPARED = "firstName,lastName,dateOfBirth,embassy"

interface DecisionRecord {
    rule: number | null
    conditions: Record<string, unknown>
    decision: IApplicationsStatus
    reason: string | null
    identity: "VERIFIED" | "MANUAL_ENTRY"
    voter_found?: boolean
    /** The compared fields that differ from the registry voter. */
    differs?: string[]
}

/** The `decision` annotation Harvest stores with an application the matrix decided. */
export function decisionRecord({
    rule,
    conditions,
    decision,
    reason,
    identity,
    voter_found = true,
    differs = [],
}: DecisionRecord) {
    return {
        matrix_version: 1,
        matrix_source: "BUILT_IN",
        rule,
        conditions,
        decision,
        reason,
        inputs: {
            identity,
            voter_found,
            already_enrolled: false,
            valid_id: null,
            fields: voter_found
                ? Object.fromEntries(
                      COMPARED.split(",").map((field) => [
                          field,
                          differs.includes(field) ? "DIFFERS" : "MATCHES",
                      ])
                  )
                : {},
            differing: differs.length,
        },
        candidates: voter_found ? 1 : 0,
        accepted_candidates: voter_found ? 1 : 0,
    }
}

/** Alice's enrollment, with the given changes. */
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
        annotations: {
            "search-attributes": COMPARED,
            "unset-attributes": "email",
            "decision": decisionRecord({
                rule: 5,
                conditions: {differing: "exactly_1", fields: {embassy: "MATCHES"}},
                decision: IApplicationsStatus.PENDING,
                reason: "NO_VOTER",
                identity: "VERIFIED",
                differs: ["dateOfBirth"],
            }),
        },
        labels: {},
        permission_label: null,
        status: IApplicationsStatus.PENDING,
        verification_type: "MANUAL",
        created_at: "2026-01-15T12:00:00.000Z",
        updated_at: "2026-01-15T12:00:00.000Z",
        ...overrides,
    }
}

/** The enrollments of the event: two that wait for a person, an approved and a rejected one. */
export const applications = (): StoryRecord<Sequent_Backend_Applications>[] => [
    applicationRecord(),
    applicationRecord({
        id: SECOND_APPLICATION_ID,
        applicant_id: "applicant-0002",
        applicant_data: {
            firstName: "Bob",
            lastName: "Example",
            email: "bob@example.test",
            dateOfBirth: "1985-02-03",
            embassy: "Lisbon",
        },
        annotations: {
            "search-attributes": COMPARED,
            "unset-attributes": "email",
            "verified_by": "admin",
            "decision": decisionRecord({
                rule: 5,
                conditions: {differing: "exactly_1", fields: {embassy: "MATCHES"}},
                decision: IApplicationsStatus.PENDING,
                reason: "NO_VOTER",
                identity: "VERIFIED",
                differs: ["firstName"],
            }),
        },
        status: IApplicationsStatus.ACCEPTED,
        created_at: "2026-01-12T09:30:00.000Z",
        updated_at: "2026-01-13T10:00:00.000Z",
    }),
    applicationRecord({
        id: MANUAL_APPLICATION_ID,
        area_id: STORY_IDS.secondArea,
        applicant_id: "applicant-0003",
        applicant_data: {
            firstName: "Carol",
            lastName: "Sample",
            email: "carol@example.test",
            dateOfBirth: "1978-11-30",
            embassy: "Rome",
        },
        annotations: {
            "search-attributes": COMPARED,
            "unset-attributes": "email",
            "decision": decisionRecord({
                rule: 2,
                conditions: {identity: "MANUAL_ENTRY"},
                decision: IApplicationsStatus.PENDING,
                reason: "IDENTITY_NOT_VERIFIED",
                identity: "MANUAL_ENTRY",
            }),
        },
        created_at: "2026-01-14T08:00:00.000Z",
        updated_at: "2026-01-14T08:00:00.000Z",
    }),
    applicationRecord({
        id: REJECTED_APPLICATION_ID,
        area_id: null,
        applicant_id: "applicant-0004",
        applicant_data: {
            firstName: "Dan",
            lastName: "Nobody",
            email: "dan@example.test",
            dateOfBirth: "1969-07-20",
            embassy: "Paris",
        },
        annotations: {
            "search-attributes": COMPARED,
            "unset-attributes": "email",
            "rejection_reason": "NO_VOTER",
            "decision": decisionRecord({
                rule: null,
                conditions: {},
                decision: IApplicationsStatus.REJECTED,
                reason: "NO_VOTER",
                identity: "VERIFIED",
                voter_found: false,
            }),
        },
        status: IApplicationsStatus.REJECTED,
        verification_type: "AUTOMATIC",
        created_at: "2026-01-11T16:45:00.000Z",
        updated_at: "2026-01-11T16:45:00.000Z",
    }),
]

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

/**
 * The voter registry. Enrolling sets a voter's email, so the voters who have
 * one are already enrolled: Bob is, and Alice, Alicia and Carol are not.
 */
export const REGISTRY_VOTERS = [
    {
        id: STORY_IDS.user,
        username: "alice.example",
        first_name: "Alice",
        last_name: "Example",
        email: null,
        enabled: true,
        email_verified: false,
        attributes: {dateOfBirth: ["1990-05-07"], embassy: ["Madrid"]},
    },
    {
        id: STORY_IDS.secondUser,
        username: "alicia.example",
        first_name: "Alicia",
        last_name: "Example",
        email: null,
        enabled: true,
        email_verified: false,
        attributes: {dateOfBirth: ["1991-06-18"], embassy: ["Lisbon"]},
    },
    {
        id: CAROL_VOTER_ID,
        username: "carol.sample",
        first_name: "Carol",
        last_name: "Sample",
        email: null,
        enabled: true,
        email_verified: false,
        attributes: {dateOfBirth: ["1978-11-30"], embassy: ["Rome"]},
    },
    {
        id: BOB_VOTER_ID,
        username: "bob.example",
        first_name: "Robert",
        last_name: "Example",
        email: "bob@example.test",
        enabled: true,
        email_verified: true,
        attributes: {dateOfBirth: ["1985-02-03"], embassy: ["Lisbon"]},
    },
]

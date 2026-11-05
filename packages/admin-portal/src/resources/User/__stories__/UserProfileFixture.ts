// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A synthetic Keycloak user profile shared by the voter editor stories.
import type {UserProfileAttribute, UserProfileAttributeGroup} from "@/gql/graphql"

const attribute = (
    name: string,
    display_name: string,
    extra: Partial<UserProfileAttribute> = {}
): UserProfileAttribute => ({
    name,
    display_name,
    annotations: {},
    validations: {},
    group: null,
    multivalued: false,
    permissions: null,
    required: null,
    selector: null,
    ...extra,
})

export const PROFILE_GROUPS: UserProfileAttributeGroup[] = [
    {
        name: "personal",
        display_header: "Personal data",
        display_description: "Identity of the voter",
        annotations: {},
    },
    {name: "address", display_header: "Address", display_description: "", annotations: {}},
]

export const PROFILE_ATTRIBUTES: UserProfileAttribute[] = [
    attribute("username", "Username", {required: {roles: ["admin"]}}),
    attribute("email", "Email"),
    attribute("first_name", "First name", {group: "personal"}),
    attribute("date-of-birth", "Date of birth", {
        group: "personal",
        annotations: {inputType: "html5-date"},
    }),
    attribute("city", "City", {group: "address", validations: {length: {min: 2, max: 20}}}),
]

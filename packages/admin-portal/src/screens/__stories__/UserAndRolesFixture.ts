// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A synthetic user of the tenant realm and its profile configuration.
import type {IUser} from "@sequentech/ui-core"
import {storyId} from "@/__stories__/fixtures"

export type UserRecord = IUser & {id: string}

export const TENANT_USER_ID = storyId(3, 9)

export const userRecords = (): UserRecord[] => [
    {
        id: TENANT_USER_ID,
        username: "alice",
        email: "alice@admin-story.invalid",
        first_name: "Alice",
        last_name: "Admin",
        enabled: true,
        email_verified: true,
        attributes: {},
        votes_info: [],
    },
]

/** Only the username is configured, as a realm without custom attributes has. */
export const userProfileConfiguration = () => ({
    data: {
        get_user_profile_configuration: {
            attributes: [
                {
                    name: "username",
                    display_name: "Username",
                    multivalued: false,
                    annotations: {},
                    validations: {},
                    group: null,
                    required: null,
                    permissions: null,
                    selector: null,
                },
            ],
            groups: [],
        },
    },
})

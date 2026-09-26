// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The user services and records behind the voter and user editors.
import {graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import {STORY_IDS, areaRecords, electionRecord} from "@/__stories__/fixtures"

/** What the user service answers to a create or an edit. */
export type UserServiceOutcome = "saved" | "failure" | "passwordFailure"

export function userServices(outcome: UserServiceOutcome, activeRoleIds: string[] = []) {
    return graphqlBoundary(
        {
            CreateUser: () => {
                if (outcome === "failure") throw new Error("Synthetic user service failure")
                return {
                    data: {
                        create_user: {
                            id: STORY_IDS.secondUser,
                            attributes: {},
                            email: null,
                            email_verified: false,
                            enabled: true,
                            first_name: null,
                            last_name: null,
                            username: "bob",
                        },
                    },
                }
            },
            EditUser: ({variables}) => {
                const body = (variables as {body: {password?: string}}).body
                if (outcome === "failure" || (outcome === "passwordFailure" && body.password))
                    throw new Error("Synthetic user service failure")
                return {data: {edit_user: {user: null, task_execution: null}}}
            },
            ListUserRoles: () => ({
                data: {list_user_roles: activeRoleIds.map((id) => ({id}))},
            }),
            SetUserRole: () => ({data: {set_user_role: {id: STORY_IDS.user}}}),
            DeleteUserRole: () => ({data: {delete_user_role: {id: STORY_IDS.user}}}),
        },
        {schema: true}
    )
}

/** The voter's areas, elections and cast votes; the voter has not voted. */
export const userRecords = () =>
    resourceBoundary({
        sequent_backend_area: areaRecords(),
        sequent_backend_election: [electionRecord()],
        sequent_backend_cast_vote: [],
    })

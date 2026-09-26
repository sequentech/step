// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {expect, type Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {TENANT_ID} from "../fixtures"

export const AREA_ID = "44444444-4444-4444-8444-444444444444"
export const ALICE_ID = "33333333-3333-4333-8333-333333333301"
export const BOB_ID = "33333333-3333-4333-8333-333333333302"
export const DOCUMENT_ID = "55555555-5555-4555-8555-555555555501"
export const TASK_ID = "66666666-6666-4666-8666-666666666601"

/** The token of an operator who can only reach the event's Voters tab. */
export const VOTER_TAB_ROLES = [
    "admin-user",
    "election-event-read",
    "election-read",
    "election-event-voters-tab",
    "voter-read",
    "area-read",
    "cast-vote-read",
]

export interface ProfileAttribute {
    name: string
    display_name: string
    multivalued: boolean
    annotations: Record<string, unknown>
    validations: Record<string, unknown>
    group: string | null
    required: Record<string, unknown> | null
    permissions: Record<string, unknown> | null
    selector: Record<string, unknown> | null
}

export const attribute = (
    name: string,
    overrides: Partial<ProfileAttribute> = {}
): ProfileAttribute => ({
    name,
    display_name: name,
    multivalued: false,
    annotations: {},
    validations: {},
    group: null,
    required: null,
    permissions: null,
    selector: null,
    ...overrides,
})

export const BASIC_ATTRIBUTES = [
    attribute("username", {display_name: "Username"}),
    attribute("email", {display_name: "Email"}),
]

export function electionEvent(overrides: Record<string, unknown> = {}) {
    return {
        id: IDS.event,
        tenant_id: TENANT_ID,
        name: "Council",
        description: "Council election",
        encryption_protocol: "RSA256",
        is_archived: false,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        elections: [],
        elections_aggregate: {aggregate: {count: 0}, nodes: []},
        presentation: {
            i18n: {en: {name: "Council"}},
            language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
        },
        status: {},
        voting_channels: {online: true},
        ...overrides,
    }
}

export function user(id: string, username: string, overrides: Record<string, unknown> = {}) {
    return {
        id,
        username,
        email: `${username}@example.test`,
        enabled: true,
        email_verified: false,
        first_name: null,
        last_name: null,
        attributes: {},
        area: null,
        groups: [],
        votes_info: [],
        ...overrides,
    }
}

/** A `sequent_backend_tasks_execution` row as `GetTaskById` selects it. */
export function taskRow(
    type: string,
    status: string,
    annotations: Record<string, unknown> = {},
    id = TASK_ID
) {
    return {
        id,
        election_event_id: IDS.event,
        tenant_id: TENANT_ID,
        execution_status: status,
        type,
        start_at: FIXED_TIME,
        end_at: status === "IN_PROGRESS" ? null : FIXED_TIME,
        logs: [{created_date: FIXED_TIME, log_text: `Synthetic ${type} ${status}`}],
        annotations,
        executed_by_user: "synthetic-admin",
    }
}

/** A `task_execution` object as Harvest actions return it. */
export function taskExecution(type: string, id = TASK_ID) {
    return {
        id,
        name: type,
        execution_status: "IN_PROGRESS",
        created_at: FIXED_TIME,
        start_at: FIXED_TIME,
        end_at: null,
        logs: [],
        annotations: {},
        labels: {},
        executed_by_user: "synthetic-admin",
        tenant_id: TENANT_ID,
        election_event_id: IDS.event,
        type,
    }
}

export interface VotersOptions {
    event?: Record<string, unknown>
    attributes?: ProfileAttribute[]
    users?: Record<string, unknown>[]
    castVotes?: Record<string, unknown>[]
    total?: number
}

/** Answers what the event page and its Voters tab read; `users` is the live list. */
export function mockVoters(portal: PortalServices, options: VotersOptions = {}) {
    const event = electionEvent(options.event)
    const state = {users: options.users ?? [user(ALICE_ID, "alice")]}
    const area = {
        id: AREA_ID,
        tenant_id: TENANT_ID,
        election_event_id: IDS.event,
        name: "North",
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }
    portal.graphql.on("sequent_backend_election_event", () => ({
        data: {
            sequent_backend_election_event: [event],
            sequent_backend_election_event_aggregate: {aggregate: {count: 1}},
        },
    }))
    portal.graphql.on("election_events_tree", () => ({
        data: {sequent_backend_election_event: [event]},
    }))
    portal.graphql.on("election_tree", () => ({data: {sequent_backend_election: []}}))
    portal.graphql.on("GetUserProfileConfiguration", () => ({
        data: {
            get_user_profile_configuration: {
                attributes: options.attributes ?? BASIC_ATTRIBUTES,
                groups: [],
            },
        },
    }))
    portal.graphql.on("getUsers", () => ({
        data: {
            get_users: {
                items: state.users,
                total: {aggregate: {count: options.total ?? state.users.length}},
            },
        },
    }))
    portal.graphql.on("getRoles", () => ({
        data: {get_roles: {items: [], total: {aggregate: {count: 0}}}},
    }))
    portal.graphql.on("ListUserRoles", () => ({data: {list_user_roles: []}}))
    portal.graphql.on("sequent_backend_area", () => ({
        data: {
            sequent_backend_area: [area],
            sequent_backend_area_aggregate: {aggregate: {count: 1}},
        },
    }))
    portal.graphql.on("sequent_backend_election", () => ({
        data: {
            sequent_backend_election: [],
            sequent_backend_election_aggregate: {aggregate: {count: 0}},
        },
    }))
    portal.graphql.on("sequent_backend_cast_vote", () => ({
        data: {
            sequent_backend_cast_vote: options.castVotes ?? [],
            sequent_backend_cast_vote_aggregate: {
                aggregate: {count: options.castVotes?.length ?? 0},
            },
        },
    }))
    return state
}

export async function openVoters(page: Page, portal: PortalServices) {
    await page.goto(`${portal.origin}/sequent_backend_election_event/${IDS.event}?lang=en`)
    await expect(page.getByRole("tab", {name: "Voters", exact: true})).toBeVisible()
    await expect.poll(() => portal.graphql.callsTo("getUsers").length).toBeGreaterThan(0)
}

/** Every call to `operation` carried `role` as its Hasura role. */
export function expectRole(portal: PortalServices, operation: string, role: string) {
    const calls = portal.graphql.callsTo(operation)
    expect(calls.length, `${operation} was requested`).toBeGreaterThan(0)
    expect(
        calls.map((call) => call.headers["x-hasura-role"]),
        `${operation} role`
    ).toEqual(calls.map(() => role))
}

/** Opens the row actions menu of the only voter and picks `name`. */
export async function rowAction(page: Page, name: string) {
    await page.getByRole("button", {name: "Actions", exact: true}).click()
    await page.getByRole("menuitem", {name, exact: true}).click()
}

export const OPERATOR_ROLE_ID = "88888888-8888-4888-8888-888888888801"
export const AUDITOR_ROLE_ID = "88888888-8888-4888-8888-888888888802"

export function role(id: string, name: string, permissions: string[]) {
    return {
        id,
        name,
        permissions,
        access: {view: true, manage: true},
        attributes: {},
        client_roles: {},
    }
}

export function permission(name: string) {
    return {
        id: `permission-${name}`,
        attributes: {},
        container_id: "tenant-realm",
        description: null,
        name,
    }
}

export interface TenantScreenOptions {
    users?: Record<string, unknown>[]
    roles?: ReturnType<typeof role>[]
    permissions?: ReturnType<typeof permission>[]
    userRoles?: ReturnType<typeof role>[]
    attributes?: ProfileAttribute[]
}

/** Answers what `/user-roles` reads; `users` and `roles` are the live lists. */
export function mockTenantScreen(portal: PortalServices, options: TenantScreenOptions = {}) {
    const state = {
        users: options.users ?? [user(ALICE_ID, "alice")],
        roles: options.roles ?? [
            role(OPERATOR_ROLE_ID, "operator", ["voter-read"]),
            role(AUDITOR_ROLE_ID, "auditor", ["logs-read"]),
        ],
    }
    portal.graphql.on("GetUserProfileConfiguration", () => ({
        data: {
            get_user_profile_configuration: {
                attributes: options.attributes ?? BASIC_ATTRIBUTES,
                groups: [],
            },
        },
    }))
    portal.graphql.on("getUsers", () => ({
        data: {
            get_users: {items: state.users, total: {aggregate: {count: state.users.length}}},
        },
    }))
    portal.graphql.on("getRoles", () => ({
        data: {get_roles: {items: state.roles, total: {aggregate: {count: state.roles.length}}}},
    }))
    portal.graphql.on("getPermissions", () => {
        const items = options.permissions ?? [
            permission("voter-read"),
            permission("voter-write"),
            permission("logs-read"),
        ]
        return {data: {get_permissions: {items, total: {aggregate: {count: items.length}}}}}
    })
    portal.graphql.on("ListUserRoles", () => ({
        data: {list_user_roles: options.userRoles ?? []},
    }))
    return state
}

export async function openUsersAndRoles(page: Page, portal: PortalServices) {
    await page.goto(`${portal.origin}/user-roles?lang=en`)
    await expect(page.getByText("Users and Roles", {exact: true})).toBeVisible()
}

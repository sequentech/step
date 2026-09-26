// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "./fixtures"

const USER_ID = "33333333-3333-4333-8333-333333333333"
const AREA_ID = "44444444-4444-4444-8444-444444444444"
const readerRoles = [
    "admin-user",
    "election-event-read",
    "election-read",
    "election-event-voters-tab",
    "voter-read",
]
const writerRoles = [
    ...readerRoles,
    "voter-create",
    "voter-write",
    "voter-delete",
    "area-read",
    "cast-vote-read",
    "ee-voters-columns",
]
test.use({roles: writerRoles})
function voters(portal: PortalServices, existing = false) {
    const event = {
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
    }
    const area = {
        id: AREA_ID,
        tenant_id: TENANT_ID,
        election_event_id: IDS.event,
        name: "North",
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
    }
    let user: Record<string, unknown> | undefined = existing
        ? {
              id: USER_ID,
              username: "alice",
              email: "alice@example.test",
              enabled: true,
              email_verified: false,
              attributes: {"area-id": [AREA_ID]},
              area: {id: AREA_ID, name: "North"},
              groups: [],
              votes_info: [],
              first_name: null,
              last_name: null,
          }
        : undefined
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
                attributes: ["username", "email"].map((name) => ({
                    name,
                    display_name: name === "username" ? "Username" : "Email",
                    multivalued: false,
                    annotations: {},
                    validations: {},
                    group: null,
                    required: null,
                    permissions: null,
                    selector: null,
                })),
                groups: [],
            },
        },
    }))
    portal.graphql.on("getUsers", () => ({
        data: {get_users: {items: user ? [user] : [], total: {aggregate: {count: user ? 1 : 0}}}},
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
            sequent_backend_cast_vote: [],
            sequent_backend_cast_vote_aggregate: {aggregate: {count: 0}},
        },
    }))
    portal.graphql.on("CreateUser", ({variables}) => {
        user = {
            ...(variables.user as Record<string, unknown>),
            id: USER_ID,
            email_verified: false,
            area: {id: AREA_ID, name: "North"},
            groups: [],
            votes_info: [],
            first_name: null,
            last_name: null,
        }
        return {data: {create_user: user}}
    })
    portal.graphql.on("EditUser", ({variables}) => {
        const body = variables.body as Record<string, unknown>
        user = {...user, email: body.email, enabled: body.enabled, attributes: body.attributes}
        return {data: {edit_user: {user, task_execution: null}}}
    })
    portal.graphql.on("DeleteUser", () => {
        user = undefined
        return {data: {delete_user: {id: USER_ID}}}
    })
}
async function boot(page: Page, portal: PortalServices) {
    await page.goto(`${portal.origin}/sequent_backend_election_event/${IDS.event}?lang=en`)
    await expect(page.getByRole("tab", {name: "Voters", exact: true})).toBeVisible()
    await expect.poll(() => portal.graphql.callsTo("getUsers").length).toBeGreaterThan(0)
}

test("creates, reviews an edit, and deletes an event voter with scoped mutations", async ({
    page,
    portal,
}) => {
    voters(portal)
    await boot(page, portal)
    await page.getByText("Create Voter", {exact: true}).click()
    const drawer = page.getByRole("dialog")
    await drawer.getByRole("textbox", {name: "Username", exact: true}).fill("alice")
    await drawer.getByRole("textbox", {name: "Email", exact: true}).fill("alice@example.test")
    await drawer.getByRole("combobox", {name: "Area"}).click()
    await page.getByRole("option", {name: "North", exact: true}).click()
    await drawer.getByRole("button", {name: "Save", exact: true}).click()
    await expect(drawer).not.toBeVisible()
    await expect(page.getByRole("cell", {name: "alice", exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("CreateUser")[0].variables).toEqual({
        tenantId: TENANT_ID,
        electionEventId: IDS.event,
        user: {
            username: "alice",
            email: "alice@example.test",
            enabled: true,
            attributes: {"area-id": [AREA_ID]},
        },
        userRolesIds: [],
        secretAttributes: {},
    })
    await page.getByRole("button", {name: "Columns", exact: true}).click()
    await page.getByRole("switch", {name: "Email", exact: true}).check()
    await page.keyboard.press("Escape")
    await expect(page.getByRole("cell", {name: "alice@example.test", exact: true})).toBeVisible()
    await page.getByRole("button", {name: "Actions", exact: true}).click()
    await page.getByRole("menuitem", {name: "Edit", exact: true}).click()
    await expect(drawer.getByRole("textbox", {name: "Username", exact: true})).toBeDisabled()
    await drawer.getByRole("textbox", {name: "Email", exact: true}).fill("updated@example.test")
    await drawer.getByRole("button", {name: "Save", exact: true}).click()
    await expect(drawer.getByRole("button", {name: "Confirm changes", exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("EditUser")).toHaveLength(0)
    await expect(drawer.getByText("alice@example.test", {exact: true})).toBeVisible()
    await expect(drawer.getByText("updated@example.test", {exact: true})).toBeVisible()
    const refreshedUsers = page.waitForResponse(
        (response) =>
            response.request().method() === "POST" &&
            response.request().postDataJSON()?.operationName === "getUsers"
    )
    await drawer.getByRole("button", {name: "Confirm changes", exact: true}).click()
    await (await refreshedUsers).finished()
    await expect(page.getByRole("cell", {name: "updated@example.test", exact: true})).toBeVisible()
    await expect(drawer).not.toBeVisible()
    expect(portal.graphql.callsTo("EditUser")[0].variables).toEqual({
        body: {
            user_id: USER_ID,
            tenant_id: TENANT_ID,
            election_event_id: IDS.event,
            first_name: null,
            last_name: null,
            email: "updated@example.test",
            enabled: true,
            temporary: true,
            attributes: {"area-id": [AREA_ID]},
            secret_attributes: {},
        },
    })
    // The refreshed email is visible before reopening the menu, so its list cannot remount mid-click.
    await page.getByRole("button", {name: "Actions", exact: true}).click()
    await page.getByRole("menuitem", {name: "Edit", exact: true}).click()
    await expect(drawer.getByRole("textbox", {name: "Email", exact: true})).toHaveValue(
        "updated@example.test"
    )
    await page.keyboard.press("Escape")
    await expect(drawer).not.toBeVisible()
    expect(portal.graphql.callsTo("EditUser")).toHaveLength(1)
    await page.getByRole("button", {name: "Actions", exact: true}).click()
    await page.getByRole("menuitem", {name: "Delete", exact: true}).click()
    await drawer.getByRole("button", {name: "Cancel", exact: true}).click()
    expect(portal.graphql.callsTo("DeleteUser")).toHaveLength(0)
    await page.getByRole("button", {name: "Actions", exact: true}).click()
    await page.getByRole("menuitem", {name: "Delete", exact: true}).click()
    await drawer.getByRole("button", {name: "Delete", exact: true}).click()
    await expect(page.getByRole("cell", {name: "alice", exact: true})).not.toBeVisible()
    expect(portal.graphql.callsTo("DeleteUser")[0].variables).toEqual({
        tenantId: TENANT_ID,
        electionEventId: IDS.event,
        userId: USER_ID,
    })
    for (const operation of [
        "CreateUser",
        "EditUser",
        "DeleteUser",
        "getUsers",
        "getRoles",
        "GetUserProfileConfiguration",
    ]) {
        expect(
            portal.graphql.callsTo(operation).length,
            `${operation} was requested`
        ).toBeGreaterThan(0)
        expect(
            portal.graphql
                .callsTo(operation)
                .every((call) => call.headers["x-hasura-role"] === "admin-user"),
            `${operation} role`
        ).toBe(true)
    }
    for (const [operation, role] of [
        ["sequent_backend_area", "area-read"],
        ["sequent_backend_cast_vote", "cast-vote-read"],
    ]) {
        expect(portal.graphql.callsTo(operation).length).toBeGreaterThan(0)
        for (const call of portal.graphql.callsTo(operation))
            expect(call.headers["x-hasura-role"]).toBe(role)
    }
    for (const call of portal.graphql.callsTo("getUsers"))
        expect(call.variables).toMatchObject({
            tenant_id: TENANT_ID,
            election_event_id: IDS.event,
            showVotesInfo: true,
        })
})

test.describe("restricted voter token", () => {
    test.use({roles: readerRoles})
    test("keeps the voter list readable while hiding every mutation action", async ({
        page,
        portal,
    }) => {
        voters(portal, true)
        await boot(page, portal)
        await expect(page.getByRole("cell", {name: "alice", exact: true})).toBeVisible()
        for (const name of ["Add", "Import", "Export", "Actions"])
            await expect(page.getByRole("button", {name, exact: true})).not.toBeVisible()
        for (const operation of [
            "CreateUser",
            "EditUser",
            "DeleteUser",
            "ImportUsers",
            "ExportUsers",
        ])
            expect(portal.graphql.callsTo(operation)).toHaveLength(0)
        expect(portal.graphql.callsTo("getUsers")[0].variables).toMatchObject({
            tenant_id: TENANT_ID,
            election_event_id: IDS.event,
        })
        const claims = portal.oidc.verifyAccessToken(portal.oidc.lastIssued()!.access_token)!
        expect(claims["https://hasura.io/jwt/claims"]).toMatchObject({
            "x-hasura-allowed-roles": readerRoles,
        })
    })
})

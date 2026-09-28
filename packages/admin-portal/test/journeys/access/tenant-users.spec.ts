// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {
    ALICE_ID,
    AUDITOR_ROLE_ID,
    BOB_ID,
    DOCUMENT_ID,
    OPERATOR_ROLE_ID,
    expectRole,
    mockTenantScreen,
    openUsersAndRoles,
    role,
    user,
} from "./data"

// Tenant-user writes require user permissions, independent of voter permissions.
const TENANT_ADMIN_ROLES = [
    "admin-user",
    "users-menu",
    "user-read",
    "user-create",
    "user-write",
    "user-import",
    "role-read",
    "role-assign",
    "voter-export",
]

test.describe("tenant administrator", () => {
    test.use({roles: TENANT_ADMIN_ROLES})

    test("lists tenant users without voter scope or vote information", async ({page, portal}) => {
        mockTenantScreen(portal, {users: [user(ALICE_ID, "alice"), user(BOB_ID, "bob")]})
        await openUsersAndRoles(page, portal)
        await expect(page.getByRole("tab", {name: "Users", exact: true})).toBeVisible()
        await expect(page.getByRole("tab", {name: "Roles", exact: true})).toBeVisible()
        await expect(page.getByRole("cell", {name: "bob", exact: true})).toBeVisible()
        for (const name of ["Add", "Import", "Export"])
            await expect(page.getByRole("button", {name, exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("getUsers")[0].variables).toMatchObject({
            tenant_id: TENANT_ID,
            election_event_id: null,
            election_id: null,
            showVotesInfo: false,
        })
        expect(portal.graphql.callsTo("GetUserProfileConfiguration")[0].variables).toEqual({
            tenantId: TENANT_ID,
        })
        expect(portal.graphql.callsTo("getRoles")[0].variables).toMatchObject({
            tenant_id: TENANT_ID,
        })
        expectRole(portal, "getUsers", "admin-user")
    })

    test("opens the requested administrator from a direct edit link", async ({page, portal}) => {
        const alice = user(ALICE_ID, "alice")
        const bob = user(BOB_ID, "bob")
        mockTenantScreen(portal, {users: [alice]})
        portal.graphql.on("getUsers", ({variables}) => ({
            data: {
                get_users: {
                    items: variables.userIds ? [bob] : [alice],
                    total: {aggregate: {count: 1}},
                },
            },
        }))
        await page.goto(`${portal.origin}/user/${BOB_ID}?lang=en`)
        const drawer = page.getByRole("dialog")
        await expect(drawer.getByRole("textbox", {name: "Email", exact: true})).toHaveValue(
            "bob@example.test"
        )
        expect(
            portal.graphql
                .callsTo("getUsers")
                .filter((call) => call.variables.userIds)
                .map((call) => call.variables)
        ).toEqual([
            {
                tenant_id: TENANT_ID,
                election_event_id: null,
                election_id: null,
                userIds: [BOB_ID],
                showVotesInfo: false,
                limit: 1,
                offset: 0,
                email: null,
                username: null,
                first_name: null,
                last_name: null,
                attributes: null,
                enabled: null,
                email_verified: null,
                has_voted: null,
                sort: {"'field'": "id", "'order'": "ASC"},
            },
        ])
        expect(portal.graphql.callsTo("ListUserRoles").map((call) => call.variables)).toEqual([
            {tenantId: TENANT_ID, userId: BOB_ID},
        ])
        await page.keyboard.press("Escape")
        await expect(drawer).toHaveCount(0)
        await expect(page).toHaveURL(`${portal.origin}/user`)
        await expect(page.getByRole("cell", {name: "alice", exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("EditUser")).toHaveLength(0)
    })

    test("creates an administrator with a role and a temporary password", async ({
        page,
        portal,
    }) => {
        const state = mockTenantScreen(portal, {users: []})
        portal.graphql.on("CreateUser", ({variables}) => {
            const created = user(BOB_ID, "bob")
            state.users.push(created)
            return {data: {create_user: {...created, ...(variables.user as object)}}}
        })
        portal.graphql.on("EditUser", () => ({
            data: {edit_user: {user: {id: BOB_ID}, task_execution: null}},
        }))
        await openUsersAndRoles(page, portal)
        await page.getByRole("button", {name: /Create user$/}).click()
        const drawer = page.getByRole("dialog")
        await drawer.getByRole("textbox", {name: "Username", exact: true}).fill("bob")
        await drawer.getByRole("textbox", {name: "Email", exact: true}).fill("bob@example.test")
        await drawer.locator('input[name="password"]').fill("Initial-Pass-2026")
        await drawer.locator('input[name="confirm_password"]').fill("Initial-Pass-2026")
        await drawer
            .getByRole("row", {name: /operator/})
            .getByRole("checkbox")
            .click()
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(drawer).toHaveCount(0)
        await expect(page.getByRole("cell", {name: "bob", exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("CreateUser").map((call) => call.variables)).toEqual([
            {
                tenantId: TENANT_ID,
                user: {
                    enabled: true,
                    email: "bob@example.test",
                    username: "bob",
                    attributes: {},
                },
                userRolesIds: [OPERATOR_ROLE_ID],
                secretAttributes: {},
            },
        ])
        expect(portal.graphql.callsTo("EditUser").map((call) => call.variables)).toEqual([
            {
                body: {
                    user_id: BOB_ID,
                    tenant_id: TENANT_ID,
                    password: "Initial-Pass-2026",
                    temporary: true,
                },
            },
        ])
        expectRole(portal, "CreateUser", "admin-user")
    })

    test("labels an empty tenant user list in words", async ({page, portal}) => {
        mockTenantScreen(portal, {users: []})
        await openUsersAndRoles(page, portal)
        await expect(page.getByRole("button", {name: /Create user$/})).toBeVisible()

        expect(await page.getByText(/^usersAndRolesScreen\./).count()).toBe(0)
    })

    test("reviews and applies an administrator's role changes", async ({page, portal}) => {
        mockTenantScreen(portal, {
            users: [user(ALICE_ID, "alice", {first_name: "Alice", last_name: "Admin"})],
            userRoles: [role(OPERATOR_ROLE_ID, "operator", ["voter-read"])],
        })
        portal.graphql.on("EditUser", () => ({
            data: {edit_user: {user: {id: ALICE_ID}, task_execution: null}},
        }))
        portal.graphql.on("DeleteUserRole", () => ({data: {delete_user_role: {id: ALICE_ID}}}))
        portal.graphql.on("SetUserRole", () => ({data: {set_user_role: {id: ALICE_ID}}}))
        await openUsersAndRoles(page, portal)
        await page.getByRole("button", {name: "Actions", exact: true}).click()
        await page.getByRole("menuitem", {name: "Edit", exact: true}).click()
        const drawer = page.getByRole("dialog")
        const operator = drawer.getByRole("row", {name: /operator/}).getByRole("checkbox")
        await expect(operator).toBeChecked()
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(page.getByText("No changes to review", {exact: true})).toBeVisible()
        await operator.click()
        await drawer
            .getByRole("row", {name: /auditor/})
            .getByRole("checkbox")
            .click()
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        const confirm = drawer.getByRole("button", {name: "Confirm changes", exact: true})
        await expect(confirm).toBeVisible()
        expect(portal.graphql.callsTo("EditUser")).toHaveLength(0)
        await confirm.click()
        await expect(drawer).toHaveCount(0)
        expect(portal.graphql.callsTo("EditUser").map((call) => call.variables)).toEqual([
            {
                body: {
                    user_id: ALICE_ID,
                    tenant_id: TENANT_ID,
                    first_name: "Alice",
                    last_name: "Admin",
                    enabled: true,
                    email: "alice@example.test",
                    temporary: true,
                    attributes: {},
                    secret_attributes: {},
                },
            },
        ])
        expect(portal.graphql.callsTo("DeleteUserRole").map((call) => call.variables)).toEqual([
            {tenantId: TENANT_ID, roleId: OPERATOR_ROLE_ID, userId: ALICE_ID},
        ])
        expect(portal.graphql.callsTo("SetUserRole").map((call) => call.variables)).toEqual([
            {tenantId: TENANT_ID, roleId: AUDITOR_ROLE_ID, userId: ALICE_ID},
        ])
        expect(portal.graphql.callsTo("ListUserRoles")[0].variables).toEqual({
            tenantId: TENANT_ID,
            userId: ALICE_ID,
        })
    })

    test("exports tenant users as a CSV download with the user reader role", async ({
        page,
        portal,
    }) => {
        mockTenantScreen(portal)
        const url = portal.s3.presign("documents/users.csv", "users-export")
        portal.graphql.on("ExportTenantUsers", () => ({
            data: {
                export_tenant_users: {
                    error_msg: null,
                    document_id: DOCUMENT_ID,
                    task_execution: null,
                },
            },
        }))
        portal.graphql.on("GetDocument", () => ({
            data: {sequent_backend_document: [{name: "users.csv", annotations: {}}]},
        }))
        portal.graphql.on("FetchDocument", () => ({data: {fetchDocument: {url}}}))
        await openUsersAndRoles(page, portal)
        await page.getByRole("button", {name: "Export", exact: true}).click()
        const dialog = page.getByRole("dialog")
        const downloading = page.waitForEvent("download")
        await dialog.getByRole("button", {name: "Export", exact: true}).click()
        const download = await downloading
        expect(download.url()).toBe(url)
        expect(download.suggestedFilename()).toBe("users-export.csv")
        await expect(dialog).toHaveCount(0)
        expect(portal.graphql.callsTo("ExportTenantUsers").map((call) => call.variables)).toEqual([
            {tenantId: TENANT_ID},
        ])
        expect(portal.graphql.callsTo("FetchDocument")[0].variables).toEqual({
            electionEventId: "",
            documentId: DOCUMENT_ID,
        })
        expectRole(portal, "ExportTenantUsers", "user-read")
    })

    test("deletes another administrator but never the signed-in account", async ({
        page,
        portal,
    }) => {
        const state = mockTenantScreen(portal, {
            users: [user(IDS.voter, "synthetic-admin"), user(BOB_ID, "bob")],
        })
        portal.graphql.on("DeleteUser", () => {
            state.users = state.users.filter((row) => row.id !== BOB_ID)
            return {data: {delete_user: {id: BOB_ID}}}
        })
        await openUsersAndRoles(page, portal)
        await page
            .getByRole("row", {name: /synthetic-admin/})
            .getByRole("button", {name: "Actions", exact: true})
            .click()
        await page.getByRole("menuitem", {name: "Delete", exact: true}).click()
        await expect(page.getByRole("dialog")).toHaveCount(0)
        await page
            .getByRole("row", {name: /bob/})
            .getByRole("button", {name: "Actions", exact: true})
            .click()
        await page.getByRole("menuitem", {name: "Delete", exact: true}).click()
        const dialog = page.getByRole("dialog")
        await expect(
            dialog.getByText("Are you sure you want to delete this user?", {exact: true})
        ).toBeVisible()
        await dialog.getByRole("button", {name: "Delete", exact: true}).click()
        await expect(page.getByText("User deleted", {exact: true})).toBeVisible()
        await expect(page.getByRole("cell", {name: "bob", exact: true})).toHaveCount(0)
        expect(portal.graphql.callsTo("DeleteUser").map((call) => call.variables)).toEqual([
            {tenantId: TENANT_ID, userId: BOB_ID},
        ])
    })
})

test.describe("tenant user manager without voter permissions", () => {
    test.use({roles: ["admin-user", "users-menu", "user-read", "user-create", "user-write"]})

    test("can create and edit tenant users with the user permissions", async ({page, portal}) => {
        mockTenantScreen(portal)
        await openUsersAndRoles(page, portal)
        await expect(page.getByRole("cell", {name: "alice", exact: true})).toBeVisible()

        expect(await page.getByRole("button", {name: "Add", exact: true}).count()).toBe(1)
        expect(await page.getByRole("button", {name: "Actions", exact: true}).count()).toBe(1)
    })
})

test.describe("voter manager in the tenant users screen", () => {
    test.use({
        roles: [
            "admin-user",
            "users-menu",
            "user-read",
            "voter-create",
            "voter-write",
            "voter-delete",
            "voter-email-tlf-edit",
        ],
    })

    test("voter permissions do not grant tenant-user management", async ({page, portal}) => {
        mockTenantScreen(portal)
        await openUsersAndRoles(page, portal)
        await expect(page.getByRole("cell", {name: "alice", exact: true})).toBeVisible()
        await expect(page.getByRole("button", {name: "Add", exact: true})).toHaveCount(0)
        await expect(page.getByRole("button", {name: "Actions", exact: true})).toHaveCount(0)
        await page.goto(`${portal.origin}/user/${ALICE_ID}?lang=en`)
        await expect(page.getByRole("cell", {name: "alice", exact: true})).toBeVisible()
        await expect(page.getByRole("dialog")).toHaveCount(0)
        expect(portal.graphql.callsTo("getUsers").filter((call) => call.variables.userIds)).toEqual(
            []
        )
        for (const operation of ["CreateUser", "EditUser", "DeleteUser"])
            expect(portal.graphql.callsTo(operation)).toHaveLength(0)
    })
})

test.describe("role reader without the user reader permission", () => {
    test.use({roles: ["admin-user", "users-menu", "role-read"]})

    test("sees the roles list under its only tab", async ({page, portal}) => {
        mockTenantScreen(portal)
        await openUsersAndRoles(page, portal)
        await expect(page.getByRole("tab", {name: "Roles", exact: true})).toBeVisible()
        await expect(page.getByRole("tab", {name: "Users", exact: true})).toHaveCount(0)
        await expect.poll(() => portal.graphql.calls.length).toBeGreaterThan(0)
        await expect(
            page.getByRole("cell", {name: /^(operator|alice)$/, exact: true}).first()
        ).toBeVisible()

        await expect(page.getByRole("cell", {name: "operator", exact: true})).toBeVisible({
            timeout: 1000,
        })
        expect(portal.graphql.callsTo("getUsers")).toHaveLength(0)
    })
})

test.describe("operator without the users menu", () => {
    test.use({roles: ["admin-user", "user-read", "role-read"]})

    test("is told users and roles are out of reach", async ({page, portal}) => {
        mockTenantScreen(portal)
        await page.goto(`${portal.origin}/user-roles?lang=en`)
        await expect(
            page.getByText("You don't have permission to access users or roles.", {exact: true})
        ).toBeVisible()
        expect(portal.graphql.callsTo("getUsers")).toHaveLength(0)
    })
})

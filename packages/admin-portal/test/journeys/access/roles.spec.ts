// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {test, expect, TENANT_ID} from "../fixtures"
import {OPERATOR_ROLE_ID, expectRole, mockTenantScreen, openUsersAndRoles, role} from "./data"

const ROLE_ADMIN_ROLES = [
    "admin-user",
    "users-menu",
    "user-read",
    "role-read",
    "role-create",
    "role-write",
    "user-permission-write",
]

async function openRoles(page: Page, portal: PortalServices) {
    await openUsersAndRoles(page, portal)
    await page.getByRole("tab", {name: "Roles", exact: true}).click()
    await expect(page.getByRole("cell", {name: "operator", exact: true})).toBeVisible()
}

// The row action icons carry no accessible name, only their order: edit, then delete.
const roleAction = (page: Page, name: string, action: "edit" | "delete") =>
    page
        .getByRole("row", {name: new RegExp(name)})
        .getByRole("button")
        .nth(action === "edit" ? 0 : 1)

test.describe("role administrator", () => {
    test.use({roles: ROLE_ADMIN_ROLES})

    test("creates a role with the permissions picked for it", async ({page, portal}) => {
        const state = mockTenantScreen(portal)
        portal.graphql.on("CreateRole", ({variables}) => {
            const created = role(
                "88888888-8888-4888-8888-888888888803",
                "clerk",
                (variables.role as {permissions: string[]}).permissions
            )
            state.roles.push(created)
            return {data: {create_role: created}}
        })
        await openRoles(page, portal)
        expect(portal.graphql.callsTo("getPermissions")[0].variables).toMatchObject({
            tenant_id: TENANT_ID,
        })
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog")
        await drawer.getByRole("textbox", {name: "Name", exact: true}).fill("clerk")
        await drawer
            .getByRole("row", {name: /Edit Voter/})
            .getByRole("checkbox")
            .click()
        await drawer
            .getByRole("row", {name: /Read Logs/})
            .getByRole("checkbox")
            .click()
        await drawer
            .getByRole("row", {name: /Read Logs/})
            .getByRole("checkbox")
            .click()
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(page.getByText("Role created", {exact: true})).toBeVisible()
        await expect(drawer).toHaveCount(0)
        await expect(page.getByRole("cell", {name: "clerk", exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("CreateRole").map((call) => call.variables)).toEqual([
            {
                tenantId: TENANT_ID,
                role: {
                    name: "clerk",
                    access: {
                        manage: true,
                        manageMembers: true,
                        manageMembership: true,
                        view: true,
                        viewMembers: true,
                    },
                    permissions: ["voter-write"],
                },
            },
        ])
        expectRole(portal, "CreateRole", "admin-user")
    })

    test("grants and revokes one role permission per click", async ({page, portal}) => {
        mockTenantScreen(portal)
        portal.graphql.on("SetRolePermission", () => ({
            data: {set_role_permission: {id: OPERATOR_ROLE_ID}},
        }))
        portal.graphql.on("DeleteRolePermission", () => ({
            data: {delete_role_permission: {id: OPERATOR_ROLE_ID}},
        }))
        await openRoles(page, portal)
        await roleAction(page, "operator", "edit").click()
        const drawer = page.getByRole("dialog")
        await expect(drawer.getByText("Role Data", {exact: true})).toBeVisible()
        await expect(drawer.getByRole("textbox", {name: "Name"})).toHaveValue("operator")
        const readVoter = drawer.getByRole("row", {name: /Read Voter/}).getByRole("checkbox")
        await expect(readVoter).toBeChecked()
        await drawer
            .getByRole("row", {name: /Edit Voter/})
            .getByRole("checkbox")
            .click()
        await expect(page.getByText("Permission edited", {exact: true})).toBeVisible()
        await readVoter.click()
        await expect.poll(() => portal.graphql.callsTo("DeleteRolePermission").length).toBe(1)
        expect(portal.graphql.callsTo("SetRolePermission").map((call) => call.variables)).toEqual([
            {tenantId: TENANT_ID, roleId: OPERATOR_ROLE_ID, permissionName: "voter-write"},
        ])
        expect(
            portal.graphql.callsTo("DeleteRolePermission").map((call) => call.variables)
        ).toEqual([{tenantId: TENANT_ID, roleId: OPERATOR_ROLE_ID, permissionName: "voter-read"}])
    })

    test("deletes a role only after confirmation", async ({page, portal}) => {
        const state = mockTenantScreen(portal)
        portal.graphql.on("DeleteRole", ({variables}) => {
            state.roles = state.roles.filter((row) => row.id !== variables.roleId)
            return {data: {delete_role: {id: OPERATOR_ROLE_ID}}}
        })
        await openRoles(page, portal)
        const dialog = page.getByRole("dialog")
        await roleAction(page, "operator", "delete").click()
        await expect(
            dialog.getByText("Are you sure you want to delete this role?", {exact: true})
        ).toBeVisible()
        await dialog.getByRole("button", {name: "Cancel", exact: true}).click()
        expect(portal.graphql.callsTo("DeleteRole")).toHaveLength(0)
        await roleAction(page, "operator", "delete").click()
        await dialog.getByRole("button", {name: "Delete", exact: true}).click()
        await expect(page.getByText("Role deleted", {exact: true})).toBeVisible()
        await expect(page.getByRole("cell", {name: "operator", exact: true})).toHaveCount(0)
        expect(portal.graphql.callsTo("DeleteRole").map((call) => call.variables)).toEqual([
            {tenantId: TENANT_ID, roleId: OPERATOR_ROLE_ID},
        ])
    })

    test("reports a refused role creation in words", async ({page, portal}) => {
        mockTenantScreen(portal)
        portal.graphql.on("CreateRole", () => ({
            errors: [{message: "Role already exists", extensions: {code: "Conflict"}}],
        }))
        await openRoles(page, portal)
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog")
        await drawer.getByRole("textbox", {name: "Name", exact: true}).fill("operator")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("CreateRole").length).toBe(1)
        await expect(drawer).toHaveCount(0)
        test.fail(
            true,
            "CreateRole's catch notifies the missing key usersAndRolesScreen.voters.roles.createError"
        )
        await expect(page.getByText("Error creating role", {exact: true})).toBeVisible({
            timeout: 2000,
        })
    })
})

test.describe("role reader", () => {
    test.use({roles: ["admin-user", "users-menu", "user-read", "role-read"]})

    test("cannot open role creation", async ({page, portal}) => {
        mockTenantScreen(portal)
        await openRoles(page, portal)
        await expect(page.getByRole("button", {name: "Add", exact: true})).toHaveCount(0)
    })
})

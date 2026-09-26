// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {FIXED_TIME} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {expectRole, taskExecution, taskRow} from "./data"

const NEW_TENANT_ID = "13131313-1313-4313-8313-131313131301"

test.describe("tenant creator", () => {
    test.use({roles: ["admin-user", "tenant-create", "tenant-read", "tenant-write"]})

    test("creates a tenant from the menu once the task queue accepts it", async ({
        page,
        portal,
    }) => {
        portal.graphql.once("InsertTenant", () => ({
            data: {
                insertTenant: {
                    id: NEW_TENANT_ID,
                    slug: "acme",
                    error_msg: "Failed to send task to Celery: connection refused",
                    task_execution: taskExecution("CREATE_TENANT"),
                },
            },
        }))
        portal.graphql.on("InsertTenant", () => ({
            data: {
                insertTenant: {
                    id: NEW_TENANT_ID,
                    slug: "acme",
                    error_msg: null,
                    task_execution: taskExecution("CREATE_TENANT"),
                },
            },
        }))
        portal.graphql.on("GetTaskById", () => ({
            data: {sequent_backend_tasks_execution: [taskRow("CREATE_TENANT", "SUCCESS")]},
        }))
        await page.goto(`${portal.origin}/?lang=en`)
        await expect(page.getByText("No Election Event yet", {exact: true})).toBeVisible()
        // The add-tenant control is a bare icon with neither a role nor a name.
        await page.locator(".select-tenants svg").last().click()
        const drawer = page.getByRole("dialog")
        await expect(drawer.getByText("Create new tenant", {exact: true})).toBeVisible()
        await drawer.getByRole("textbox", {name: "Slug"}).fill("acme")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("InsertTenant").length).toBe(1)
        await expect(drawer.getByRole("button", {name: "Save", exact: true})).toBeEnabled()
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(drawer).toHaveCount(0)
        await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
        expect(portal.graphql.callsTo("InsertTenant").map((call) => call.variables)).toEqual([
            {slug: "acme"},
            {slug: "acme"},
        ])
        expectRole(portal, "InsertTenant", "admin-user")
    })

    test("lists the tenant and edits its slug", async ({page, portal}) => {
        portal.graphql.on("update_sequent_backend_tenant", ({variables}) => ({
            data: {
                update_sequent_backend_tenant: {
                    affected_rows: 1,
                    returning: [
                        {
                            id: TENANT_ID,
                            slug: (variables._set as {slug: string}).slug,
                            annotations: {},
                            settings: {},
                            is_active: true,
                            labels: {},
                            voting_channels: ["ONLINE"],
                            created_at: FIXED_TIME,
                            updated_at: FIXED_TIME,
                            test: true,
                        },
                    ],
                },
            },
        }))
        await page.goto(`${portal.origin}/sequent_backend_tenant?lang=en`)
        await expect(page.getByText("Tenants", {exact: true})).toBeVisible()
        await page.getByRole("cell", {name: "synthetic", exact: true}).click()
        await expect(page).toHaveURL(new RegExp(`/sequent_backend_tenant/${TENANT_ID}`))
        const slug = page.getByRole("textbox", {name: "Slug"})
        await expect(slug).toHaveValue("synthetic")
        await slug.fill("synthetic-renamed")
        await page.getByRole("button", {name: "Save", exact: true}).click()
        await page.clock.runFor(6_000)
        await expect
            .poll(() => portal.graphql.callsTo("update_sequent_backend_tenant").length)
            .toBe(1)
        expect(portal.graphql.callsTo("update_sequent_backend_tenant")[0].variables).toEqual({
            where: {id: {_eq: TENANT_ID}},
            _set: {slug: "synthetic-renamed"},
        })
        expectRole(portal, "update_sequent_backend_tenant", "admin-user")
    })
})

async function openTenantSelection(page: Page, origin: string) {
    await page.goto(`${origin}/tenant?lang=en`)
    await expect(page.getByRole("textbox", {name: "Tenant Name"})).toBeVisible()
}

async function tryTenant(page: Page, name: string) {
    await page.getByRole("textbox", {name: "Tenant Name"}).fill(name)
    await page.getByRole("button", {name: "Continue", exact: true}).click()
}

test.describe("tenant selection", () => {
    test("shows the default tenant's logo before sign-in", async ({page, portal}) => {
        const logo = portal.s3.putBytes(
            "public",
            "tenant/logo.svg",
            new TextEncoder().encode('<svg xmlns="http://www.w3.org/2000/svg"/>'),
            "image/svg+xml"
        )
        portal.graphql.on("GetTenantById", () => ({
            data: {
                sequent_backend_tenant: [
                    {id: TENANT_ID, slug: "synthetic", annotations: {logo_url: logo}},
                ],
            },
        }))
        await openTenantSelection(page, portal.origin)
        await expect(page.getByRole("img", {name: "Logo Image"})).toHaveAttribute("src", logo)
        expect(portal.graphql.callsTo("GetTenantById")[0].variables).toEqual({id: TENANT_ID})
    })

    test("rejects an unknown tenant name", async ({page, portal}) => {
        await openTenantSelection(page, portal.origin)
        await tryTenant(page, "unknown")
        await expect(page.getByRole("alert")).toHaveText("Tenant not found")
        expect(portal.graphql.callsTo("GetTenantBySlug")[0].variables).toEqual({slug: "unknown"})
        expect(portal.oidc.authorizations).toEqual([])
    })

    test("rejects a tenant whose sign-in realm does not exist", async ({page, portal}) => {
        const orphanId = "14141414-1414-4414-8414-141414141401"
        portal.graphql.on("GetTenantBySlug", () => ({
            data: {sequent_backend_tenant: [{id: orphanId, slug: "orphan"}]},
        }))
        // The identity provider answers an unknown realm's discovery document with a 404.
        await page.route(`**/realms/tenant-${orphanId}/.well-known/openid-configuration`, (route) =>
            route.fulfill({status: 404, body: "Realm not found"})
        )
        await openTenantSelection(page, portal.origin)
        await tryTenant(page, "orphan")
        await expect(page.getByRole("alert")).toHaveText("Tenant realm not found")
        expect(portal.oidc.authorizations).toEqual([])
    })

    test("locks tenant selection after five failed attempts, across reloads", async ({
        page,
        portal,
    }) => {
        await openTenantSelection(page, portal.origin)
        for (let attempt = 1; attempt <= 5; attempt += 1) {
            await tryTenant(page, `unknown-${attempt}`)
            await expect.poll(() => portal.graphql.callsTo("GetTenantBySlug").length).toBe(attempt)
        }
        await tryTenant(page, "synthetic")
        await expect(page.getByRole("alert")).toHaveText(
            "Too many attempts. Please try again later."
        )
        await expect(page.getByRole("textbox", {name: "Tenant Name"})).toBeDisabled()
        expect(portal.graphql.callsTo("GetTenantBySlug")).toHaveLength(5)
        await page.reload()
        await expect(page.getByRole("textbox", {name: "Tenant Name"})).toBeDisabled()
        await expect(page.getByRole("alert")).toHaveText(
            "Too many attempts. Please try again later."
        )
    })

    test("asks for a tenant name when only spaces were typed", async ({page, portal}) => {
        await openTenantSelection(page, portal.origin)
        await tryTenant(page, "   ")
        await expect(page.getByRole("button", {name: "Continue", exact: true})).toBeEnabled()
        expect(portal.graphql.callsTo("GetTenantBySlug")).toHaveLength(0)
        test.fail(true, "SelectTenant sets the empty-name error without opening its snackbar")
        await expect(page.getByText("Please enter a tenant name", {exact: true})).toBeVisible({
            timeout: 2000,
        })
    })
})

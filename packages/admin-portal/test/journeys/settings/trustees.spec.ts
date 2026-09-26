// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {test, expect, TENANT_ID} from "../fixtures"
import {
    SETTINGS_ROLES,
    TRUSTEE_ID,
    mockElectionTypes,
    mockTask,
    mockTenant,
    mockTrustees,
    openSettings,
    rowWith,
    serveDocument,
    taskExecution,
    trustee,
} from "./data"

const BOB_ID = "30000000-0000-4000-8000-000000000003"
const DOCUMENT_ID = "60000000-0000-4000-8000-000000000001"

test.beforeEach(({portal}) => {
    mockTenant(portal)
    mockElectionTypes(portal)
})

test.describe("with trustee write and export permissions", () => {
    test.use({roles: [...SETTINGS_ROLES, "trustee-read", "trustee-write", "trustees-export"]})

    test("creates a trustee from the add drawer", async ({page, portal}) => {
        mockTrustees(portal, [trustee()])
        await openSettings(page, portal, "TRUSTEES")
        await expect(rowWith(page, "Trustee Alice")).toBeVisible()
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog")
        await expect(drawer.getByText("Create Trustee", {exact: true})).toBeVisible()
        await drawer.getByRole("textbox", {name: "Name", exact: true}).fill("Trustee Bob")
        await drawer.getByRole("textbox", {name: "Public key", exact: true}).fill("bob-public-key")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()

        await expect(rowWith(page, "Trustee Bob")).toBeVisible()
        await expect(page.getByRole("dialog")).toHaveCount(0)
        expect(
            portal.graphql.callsTo("insert_sequent_backend_trustee").map((c) => c.variables)
        ).toEqual([
            {objects: {tenant_id: TENANT_ID, name: "Trustee Bob", public_key: "bob-public-key"}},
        ])
    })

    test("edits a trustee's name and public key", async ({page, portal}) => {
        mockTrustees(portal, [trustee(), trustee({id: BOB_ID, name: "Trustee Bob"})])
        await openSettings(page, portal, "TRUSTEES")
        // The row actions are icon buttons without accessible names: edit first, delete last.
        await rowWith(page, "Trustee Alice").getByRole("button").first().click()
        const drawer = page.getByRole("dialog")
        await expect(drawer.getByText("Edit Trustee", {exact: true})).toBeVisible()
        const name = drawer.getByRole("textbox", {name: "Name", exact: true})
        const publicKey = drawer.getByRole("textbox", {name: "Public key", exact: true})
        await expect(name).toHaveValue("Trustee Alice")
        await expect(publicKey).toHaveValue("alice-public-key")
        await name.fill("Trustee Alicia")
        await publicKey.fill("alicia-public-key")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()

        await expect(rowWith(page, "Trustee Alicia")).toBeVisible()
        await expect(rowWith(page, "Trustee Bob")).toBeVisible()
        expect(
            portal.graphql.callsTo("update_sequent_backend_trustee").map((c) => c.variables)
        ).toEqual([
            {
                _set: {name: "Trustee Alicia", public_key: "alicia-public-key"},
                where: {id: {_eq: TRUSTEE_ID}},
            },
        ])
    })

    test("deletes a trustee only after the warning is confirmed", async ({page, portal}) => {
        mockTrustees(portal, [trustee(), trustee({id: BOB_ID, name: "Trustee Bob"})])
        await openSettings(page, portal, "TRUSTEES")
        const row = rowWith(page, "Trustee Bob")
        await row.getByRole("button").last().click()
        await page.getByRole("dialog").getByRole("button", {name: "Cancel", exact: true}).click()
        expect(portal.graphql.callsTo("delete_sequent_backend_trustee")).toEqual([])

        await row.getByRole("button").last().click()
        await expect(page.getByText("Are you sure you want to delete this item?")).toBeVisible()
        await page.getByRole("dialog").getByRole("button", {name: "Delete", exact: true}).click()
        await expect(row).toHaveCount(0)
        await expect(rowWith(page, "Trustee Alice")).toBeVisible()
        expect(
            portal.graphql.callsTo("delete_sequent_backend_trustee").map((c) => c.variables)
        ).toEqual([{where: {id: {_eq: BOB_ID}}}])
    })

    test("offers to create the first trustee from the empty state", async ({page, portal}) => {
        mockTrustees(portal)
        await openSettings(page, portal, "TRUSTEES")
        await expect(page.getByText("No Trustees yet.", {exact: true})).toBeVisible()
        await page.getByText("Create Trustee", {exact: true}).click()
        const drawer = page.getByRole("dialog")
        await drawer.getByRole("textbox", {name: "Name", exact: true}).fill("Trustee Carol")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect(rowWith(page, "Trustee Carol")).toBeVisible()
        expect(
            portal.graphql.callsTo("insert_sequent_backend_trustee").map((c) => c.variables)
        ).toEqual([{objects: {tenant_id: TENANT_ID, name: "Trustee Carol"}}])
    })

    test("exports trustees as an archive protected by the password shown", async ({
        page,
        portal,
    }) => {
        mockTrustees(portal, [trustee()])
        const task = taskExecution("EXPORT_TRUSTEES")
        portal.graphql.on("ExportTrustees", () => ({
            data: {exportTrustees: {document_id: DOCUMENT_ID, task_execution: task}},
        }))
        mockTask(portal, () => ({...task, execution_status: "SUCCESS"}))
        const {url} = serveDocument(portal, DOCUMENT_ID, "trustees-export.ezip")
        await openSettings(page, portal, "TRUSTEES")
        const download = page.waitForEvent("download")
        await page.getByRole("button", {name: "Export", exact: true}).click()

        expect((await download).suggestedFilename()).toBe("trustees-export.ezip")
        expect((await download).url()).toBe(url)
        const dialog = page.getByRole("dialog")
        await expect(dialog.getByText("Password to decrypt the file:")).toBeVisible()
        const password = await dialog.getByRole("textbox").inputValue()
        expect(password).toMatch(/^[A-Za-z0-9._-]{12}$/)
        expect(portal.graphql.callsTo("ExportTrustees").map((c) => c.variables)).toEqual([
            {password},
        ])
        expect(portal.graphql.callsTo("ExportTrustees")[0].headers["x-hasura-role"]).toBe(
            "trustees-export"
        )
        expect(portal.graphql.callsTo("GetDocument")[0].variables).toEqual({
            id: DOCUMENT_ID,
            tenantId: TENANT_ID,
        })
        expect(portal.graphql.callsTo("FetchDocument")[0].variables).toEqual({
            documentId: DOCUMENT_ID,
        })
        await expect(page.getByText("Task: Export Trustees", {exact: true})).toBeVisible()
        await expect(page.getByText("SUCCESS", {exact: true})).toBeVisible()
        await dialog.getByRole("button", {name: "Ok", exact: true}).click()
        await expect(page.getByRole("dialog")).toHaveCount(0)
    })

    test("a rejected trustee export shows no password and allows retrying", async ({
        page,
        portal,
    }) => {
        mockTrustees(portal, [trustee()])
        portal.graphql.on("ExportTrustees", () => ({errors: [{message: "export failed"}]}))
        await openSettings(page, portal, "TRUSTEES")
        const exportButton = page.getByRole("button", {name: "Export", exact: true})
        await exportButton.click()
        await expect(page.getByText("Task: Export Trustees", {exact: true})).toBeVisible()
        await expect.poll(() => portal.graphql.callsTo("ExportTrustees")).toHaveLength(1)
        await expect(exportButton).toBeEnabled()
        await expect(page.getByRole("dialog")).toHaveCount(0)
    })

    test("marks a rejected trustee export task as failed", async ({page, portal}) => {
        // Defect: Widget copies its `status` prop into state once, so updateWidgetFail never shows FAILED.
        mockTrustees(portal, [trustee()])
        let rejectExport = () => {}
        const exportReady = new Promise<void>((resolve) => (rejectExport = resolve))
        portal.graphql.on("ExportTrustees", async () => {
            await exportReady
            return {errors: [{message: "export failed"}]}
        })
        await openSettings(page, portal, "TRUSTEES")
        await page.getByRole("button", {name: "Export", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("ExportTrustees")).toHaveLength(1)
        await expect(page.getByText("IN_PROGRESS", {exact: true})).toBeVisible()
        rejectExport()
        await expect(page.getByRole("button", {name: "Export", exact: true})).toBeEnabled()
        test.fail(true, "The task widget does not reflect its updated failure status")
        await expect(page.getByText("FAILED", {exact: true})).toBeVisible({timeout: 2_000})
    })

    test("names the empty-state create button for assistive technology", async ({page, portal}) => {
        // Defect: the button wraps a nested icon button, which leaves it without an accessible name.
        mockTrustees(portal)
        await openSettings(page, portal, "TRUSTEES")
        await expect(page.getByText("No Trustees yet.", {exact: true})).toBeVisible()
        test.fail(true, "The empty-state trustee create button has no accessible name")
        await expect(page.getByRole("button", {name: "Create Trustee", exact: true})).toBeVisible({
            timeout: 2_000,
        })
    })

    test("tells the user when creating a trustee fails", async ({page, portal}) => {
        // Defect: the create drawer's onError only refreshes and closes, so the failure is silent.
        mockTrustees(portal, [trustee()])
        portal.graphql.on("insert_sequent_backend_trustee", () => ({
            errors: [{message: "Duplicate trustee public key"}],
        }))
        await openSettings(page, portal, "TRUSTEES")
        await page.getByRole("button", {name: "Add", exact: true}).click()
        const drawer = page.getByRole("dialog")
        await drawer.getByRole("textbox", {name: "Name", exact: true}).fill("Trustee Bob")
        await drawer.getByRole("button", {name: "Save", exact: true}).click()
        await expect
            .poll(() => portal.graphql.callsTo("insert_sequent_backend_trustee"))
            .toHaveLength(1)
        test.fail(true, "Trustee creation failures close the drawer without an error notification")
        await expect(page.getByText("Duplicate trustee public key")).toBeVisible({timeout: 2_000})
    })
})

test.describe("with read-only trustee access", () => {
    test.use({roles: [...SETTINGS_ROLES, "trustee-read"]})

    test("lists trustees without create or export actions", async ({page, portal}) => {
        mockTrustees(portal, [trustee()])
        await openSettings(page, portal, "TRUSTEES")
        await expect(rowWith(page, "Trustee Alice")).toBeVisible()
        await expect(page.getByRole("button", {name: "Add", exact: true})).toHaveCount(0)
        await expect(page.getByRole("button", {name: "Export", exact: true})).toHaveCount(0)
    })
})

test.describe("without trustee access", () => {
    test.use({roles: SETTINGS_ROLES})

    test("shows the empty trustees message without querying trustees", async ({page, portal}) => {
        await openSettings(page, portal, "TRUSTEES")
        await expect(page.getByText("No Trustees yet.", {exact: true})).toBeVisible()
        await expect(page.getByRole("button", {name: "Create Trustee"})).toHaveCount(0)
        expect(portal.graphql.callsTo("sequent_backend_trustee")).toEqual([])
    })
})

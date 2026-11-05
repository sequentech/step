// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, TENANT_ID, type AdminPortal} from "../fixtures"
import {
    SETTINGS_ROLES,
    mockElectionTypes,
    mockTask,
    mockTenant,
    openSettings,
    serveDocument,
    taskExecution,
} from "./data"

const DOCUMENT_ID = "70000000-0000-4000-8000-000000000001"
const CHECKSUM = "cd".repeat(32)
const ARCHIVE = Buffer.from("synthetic tenant configuration archive")
const OPTIONS = [
    "Import Tenant Configurations",
    "Import Keycloak Configurations",
    "Import Roles & Permissions Configurations",
]

test.use({roles: SETTINGS_ROLES})
test.beforeEach(({portal}) => {
    mockTenant(portal)
    mockElectionTypes(portal)
})

test("backs up the tenant configuration and downloads the archive", async ({page, portal}) => {
    const task = taskExecution("EXPORT_TENANT_CONFIG")
    portal.graphql.on("ExportTenantConfig", () => ({
        data: {
            export_tenant_config: {document_id: DOCUMENT_ID, error_msg: null, task_execution: task},
        },
    }))
    mockTask(portal, () => ({...task, execution_status: "SUCCESS"}))
    const {url} = serveDocument(portal, DOCUMENT_ID, "tenant-config.zip")
    await openSettings(page, portal, "Backup / Restore")
    await expect(page.getByText("Backup / Restore Tenant config", {exact: true})).toBeVisible()
    await expect(page.getByText("Backup Tenant configurations", {exact: true})).toBeVisible()
    const download = page.waitForEvent("download")
    await page.getByRole("button", {name: "Backup", exact: true}).click()

    expect((await download).suggestedFilename()).toBe(`tenant-config-${TENANT_ID}-export.zip`)
    expect((await download).url()).toBe(url)
    expect(portal.graphql.callsTo("ExportTenantConfig").map((c) => c.variables)).toEqual([
        {tenantId: TENANT_ID},
    ])
    expect(portal.graphql.callsTo("ExportTenantConfig")[0].headers["x-hasura-role"]).toBe(
        "tenant-read"
    )
    expect(portal.graphql.callsTo("FetchDocument")[0].variables).toEqual({documentId: DOCUMENT_ID})
    await expect(page.getByText("Task: Export Tenant Config", {exact: true})).toBeVisible()
    await expect(page.getByText("SUCCESS", {exact: true})).toBeVisible()
    await expect(page.getByRole("button", {name: "Backup", exact: true})).toBeEnabled()
})

test("lets the user retry a backup the server rejected", async ({page, portal}) => {
    portal.graphql.on("ExportTenantConfig", () => ({errors: [{message: "export unavailable"}]}))
    await openSettings(page, portal, "Backup / Restore")
    const backup = page.getByRole("button", {name: "Backup", exact: true})
    await backup.click()
    await expect(page.getByText("Task: Export Tenant Config", {exact: true})).toBeVisible()
    await expect.poll(() => portal.graphql.callsTo("ExportTenantConfig")).toHaveLength(1)
    await expect(backup).toBeEnabled()
})

test("keeps restore disabled until every configuration option is checked", async ({
    page,
    portal,
}) => {
    await openSettings(page, portal, "Backup / Restore")
    const restore = page.getByRole("button", {name: "Restore", exact: true})
    await page.getByRole("checkbox", {name: OPTIONS[0], exact: true}).check()

    await expect(restore).toBeDisabled({timeout: 2_000})
})

/** Answers the restore upload: the presigned URL and the S3 PUT to it. */
function mockRestoreUpload(portal: AdminPortal) {
    const uploadUrl = portal.s3.presign(`${TENANT_ID}/restore/tenant-config.zip`, "restore")
    portal.s3.override(
        (request) => request.method === "PUT" && request.query["X-Amz-Signature"] === "restore",
        {status: 200},
        1
    )
    portal.graphql.on("GetUploadUrl", () => ({
        data: {get_upload_url: {url: uploadUrl, document_id: DOCUMENT_ID}},
    }))
    return uploadUrl
}

async function uploadArchive(page: Page, portal: AdminPortal, url: string) {
    await openSettings(page, portal, "Backup / Restore")
    for (const option of OPTIONS) {
        await page.getByRole("checkbox", {name: option, exact: true}).check()
    }
    await page.getByRole("button", {name: "Restore", exact: true}).click()
    const drawer = page.getByRole("dialog")
    await expect(
        drawer.getByText(
            "Import tenant configurations, Keycloak configurations, roles & permissions data using zip folder",
            {exact: true}
        )
    ).toBeVisible()
    await drawer.getByRole("textbox", {name: "Integrity Check (SHA-256)"}).fill(CHECKSUM)
    const upload = page.waitForRequest(
        (request) => request.url() === url && request.method() === "PUT"
    )
    await drawer.locator('input[type="file"]').setInputFiles({
        name: "tenant-config.zip",
        mimeType: "application/zip",
        buffer: ARCHIVE,
    })
    const request = await upload
    expect(request.postDataBuffer()).toEqual(ARCHIVE)
    expect(request.headers()["content-type"]).toBe("application/zip")
    expect(portal.graphql.callsTo("GetUploadUrl").map((c) => c.variables)).toEqual([
        {
            name: "tenant-config.zip",
            media_type: "application/zip",
            size: ARCHIVE.length,
            is_public: false,
        },
    ])
    return drawer
}

test("restores an uploaded archive with every configuration option", async ({page, portal}) => {
    const url = mockRestoreUpload(portal)
    const task = taskExecution("IMPORT_TENANT_CONFIG")
    portal.graphql.on("ImportTenantConfig", () => ({
        data: {import_tenant_config: {message: "Task created", error: null, task_execution: task}},
    }))
    mockTask(portal, () => task)
    const drawer = await uploadArchive(page, portal, url)
    await expect(
        page.getByText("File uploaded to server - but not imported yet", {exact: true})
    ).toBeVisible()
    await drawer.getByRole("button", {name: "Import", exact: true}).click()

    await expect(page.getByText("Task: Import Tenant Config", {exact: true})).toBeVisible()
    await expect(page.getByRole("dialog")).toHaveCount(0)
    expect(portal.graphql.callsTo("ImportTenantConfig").map((c) => c.variables)).toEqual([
        {
            tenantId: TENANT_ID,
            documentId: DOCUMENT_ID,
            importConfigurations: {
                include_tenant: true,
                include_keycloak: true,
                include_roles: true,
            },
            sha256: CHECKSUM,
        },
    ])
    expect(portal.graphql.callsTo("ImportTenantConfig")[0].headers["x-hasura-role"]).toBe(
        "tenant-write"
    )
})

test("lets the user retry a restore the server rejected", async ({page, portal}) => {
    const url = mockRestoreUpload(portal)
    portal.graphql.on("ImportTenantConfig", () => ({errors: [{message: "import unavailable"}]}))
    const drawer = await uploadArchive(page, portal, url)
    await drawer.getByRole("button", {name: "Import", exact: true}).click()
    await expect(page.getByText("Task: Import Tenant Config", {exact: true})).toBeVisible()
    await expect.poll(() => portal.graphql.callsTo("ImportTenantConfig")).toHaveLength(1)
    await expect(page.getByRole("dialog")).toHaveCount(0)
    await expect(page.getByRole("button", {name: "Restore", exact: true})).toBeEnabled()
})

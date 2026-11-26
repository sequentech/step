// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {createCipheriv, createHash, pbkdf2Sync} from "node:crypto"
import {record} from "../../../src/resources/Settings/configurationPackage.fixture"
import {expect, TENANT_ID, test} from "../fixtures"
import {mockElectionTypes, mockTenant, openSettings, SETTINGS_ROLES} from "./data"

const DOCUMENT_ID = "70000000-0000-4000-8000-000000000141"
const TASK_ID = "70000000-0000-4000-8000-000000000142"
// Valid empty ZIP (EOCD). Only the password transport is under test: the
// interception below is not proof of a signature or of server admission.
const INNER_ZIP = Buffer.from("504b0506" + "00".repeat(18), "hex")
const SHA256 = createHash("sha256").update(INNER_ZIP).digest("hex")
const CORRECT_PASSWORD = "synthetic-fixture-only"

function encryptFixture(): Buffer {
    // Independent producer for the v1 AES-256-GCM wire format. These fixed
    // inputs are local test vectors, never used for real package encryption.
    const salt = Buffer.from("d1d2d3d4d5d6d7d8d9dadbdcdddedfe0", "hex")
    const nonce = Buffer.from("0102030405060708090a0b0c", "hex")
    const header = Buffer.concat([Buffer.from("SEQUENT-EZIP"), Buffer.of(1), salt, nonce])
    const key = pbkdf2Sync(CORRECT_PASSWORD, salt, 600_000, 32, "sha256")
    const cipher = createCipheriv("aes-256-gcm", key, nonce)
    cipher.setAAD(header)
    return Buffer.concat([header, cipher.update(INNER_ZIP), cipher.final(), cipher.getAuthTag()])
}

test.use({
    configurationPackageActions: true,
    roles: [
        ...SETTINGS_ROLES,
        "user-write",
        "role-write",
        "trustee-create",
        "trustee-write",
        "election-event-create",
    ],
})

test("unlocks an encrypted configuration in the real Worker and uploads only the byte-identical ZIP for preview", async ({
    page,
    portal,
}) => {
    mockTenant(portal)
    mockElectionTypes(portal)
    const uploadUrl = portal.s3.presign(`${TENANT_ID}/config/configuration-package.zip`, "sealed")
    portal.s3.override(
        (request) => request.method === "PUT" && request.query["X-Amz-Signature"] === "sealed",
        {status: 200},
        1
    )
    portal.graphql.on("GetUploadUrl", () => ({
        data: {get_upload_url: {url: uploadUrl, document_id: DOCUMENT_ID}},
    }))
    portal.graphql.on("ArchitectApprovalPolicy", () => ({
        data: {get_architect_approval_policy: {required_approvals: null, generation: "0"}},
    }))
    portal.graphql.on("ArchitectSignerRosterStatus", () => ({
        data: {
            get_architect_signer_roster_status: {
                __typename: "ArchitectSignerRosterStatus",
                mode: "not_enabled",
                generation: "0",
                active_sha256: null,
                required_approvals: 2,
                can_manage: false,
            },
        },
    }))
    portal.graphql.on("ListPackageApplications", () => ({
        data: {sequent_backend_tasks_execution: []},
    }))
    const annotations = record("previewed")
    if (!annotations) throw new Error("preview fixture is missing")
    annotations.application.tenantId = TENANT_ID
    annotations.application.packageSha256 = SHA256
    portal.graphql.on("ApplyConfigurationPackage", () => ({
        data: {
            apply_configuration_package: {
                task_execution: {id: TASK_ID, execution_status: "IN_PROGRESS", annotations: null},
            },
        },
    }))
    portal.graphql.on("GetPackageTask", () => ({
        data: {
            sequent_backend_tasks_execution: [
                {id: TASK_ID, execution_status: "SUCCESS", annotations, logs: []},
            ],
        },
    }))

    const uploads: string[] = []
    page.on("request", (request) => {
        if (request.method() === "PUT" && request.url() === uploadUrl) uploads.push(request.url())
    })
    await openSettings(page, portal)
    expect(await page.evaluate(() => window.isSecureContext)).toBe(true)
    await page.getByRole("tab", {name: /^configuration package$/i}).click()
    await page.getByRole("button", {name: "Upload package"}).click()
    const drawer = page.getByTestId("package-upload-drawer")
    await drawer.getByRole("textbox", {name: "Integrity Check (SHA-256)"}).fill(SHA256)
    await drawer.locator('input[type="file"]').setInputFiles({
        name: "opaque-demo-transfer.ezip",
        mimeType: "application/octet-stream",
        buffer: encryptFixture(),
    })

    const password = page.getByRole("dialog", {name: "Decryption Password"})
    await password.getByLabel("Password", {exact: true}).fill("incorrect-fixture-value")
    const badWorker = page.waitForEvent("worker", (worker) =>
        new URL(worker.url()).pathname.endsWith("/signed-delivery-worker.js")
    )
    await password.getByRole("button", {name: "Unlock package"}).click()
    expect(new URL((await badWorker).url()).origin).toBe(portal.origin)
    await expect(password.getByRole("alert")).toHaveText(/Could not unlock.*Nothing was uploaded/)
    expect(uploads).toEqual([])
    expect(portal.graphql.callsTo("GetUploadUrl")).toEqual([])
    expect(portal.graphql.callsTo("ApplyConfigurationPackage")).toEqual([])

    await password.getByLabel("Password", {exact: true}).fill(CORRECT_PASSWORD)
    const goodWorker = page.waitForEvent("worker", (worker) =>
        new URL(worker.url()).pathname.endsWith("/signed-delivery-worker.js")
    )
    const upload = page.waitForRequest(
        (request) => request.url() === uploadUrl && request.method() === "PUT"
    )
    await password.getByRole("button", {name: "Unlock package"}).click()
    expect(new URL((await goodWorker).url()).origin).toBe(portal.origin)
    const uploaded = await upload
    expect(uploaded.postDataBuffer()).toEqual(INNER_ZIP)
    expect(uploaded.headers()["content-type"]).toBe("application/zip")
    expect(uploads).toHaveLength(1)
    expect(portal.graphql.callsTo("GetUploadUrl").map(({variables}) => variables)).toEqual([
        {
            name: "configuration-package.zip",
            media_type: "application/zip",
            size: INNER_ZIP.length,
            is_public: false,
            configuration_package: true,
        },
    ])
    expect(portal.graphql.callsTo("ApplyConfigurationPackage")).toEqual([])

    await drawer.getByRole("button", {name: "Preview", exact: true}).click()
    await expect(page.getByTestId("package-task")).toBeVisible()
    expect(
        portal.graphql.callsTo("ApplyConfigurationPackage").map(({variables}) => variables)
    ).toEqual([
        expect.objectContaining({
            tenantId: TENANT_ID,
            documentId: DOCUMENT_ID,
            mode: "preview",
            encryptedPayloadSha256: SHA256,
        }),
    ])
    expect(portal.graphql.callsTo("ApplyManagedConfigurationPackage")).toEqual([])
})

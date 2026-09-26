// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, type AdminPortal} from "../fixtures"
import {
    electionEvent,
    EVENT_ID,
    EVENT_ROLES,
    FIXED_TIME,
    mockEvent,
    openEvent,
    TENANT_ID,
    type Row,
} from "./data"

const CA_ROLES = [...EVENT_ROLES, "election-event-cas-tab"]
test.use({roles: [...CA_ROLES, "ca-read", "ca-write"]})

const ROOT_ID = "c0000000-0000-4000-8000-000000000001"
const COUNCIL_ID = "c0000000-0000-4000-8000-000000000002"
const LEGACY_ID = "c0000000-0000-4000-8000-000000000003"
const IMPORTED_ID = "c0000000-0000-4000-8000-000000000004"
const TASK_ID = "90000000-0000-4000-8000-000000000001"
const ROOT_SUBJECT = "CN=Sequent Root CA,O=Sequent Tech"
const ROOT_FINGERPRINT = "3f6a9c1e0b7d2a4c5e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d"
const PEM =
    "-----BEGIN CERTIFICATE-----\nMIIBszCCAVmgAwIBAgIUCouncilVoters\n-----END CERTIFICATE-----\n"

function certificate(id: string, commonName: string, fields: Row): Row {
    return {
        id,
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        common_name: commonName,
        subject: `CN=${commonName},O=Council`,
        issuer: ROOT_SUBJECT,
        issuer_common_name: "Sequent Root CA",
        not_before: "2024-01-01T00:00:00Z",
        not_after: "2034-01-01T00:00:00Z",
        fingerprint_sha256: "0".repeat(64),
        serial_number: "01",
        pem: PEM,
        created_at: FIXED_TIME,
        ...fields,
    }
}

const ROOT = certificate(ROOT_ID, "Sequent Root CA", {
    subject: ROOT_SUBJECT,
    fingerprint_sha256: ROOT_FINGERPRINT,
})
// Expires 17 days after the fixed test time.
const COUNCIL = certificate(COUNCIL_ID, "Council Voters CA", {
    fingerprint_sha256: "b".repeat(64),
    serial_number: "4A:1F:00:9C",
    not_after: "2026-02-01T00:00:00Z",
})
const LEGACY = certificate(LEGACY_ID, "Legacy Voters CA", {
    fingerprint_sha256: "c".repeat(64),
    not_before: "2020-06-01T00:00:00Z",
    not_after: "2025-12-31T00:00:00Z",
})

/** Certificates that follow the import and delete mutations. */
function mockCertificates(portal: AdminPortal, initial: Row[] = [ROOT, COUNCIL, LEGACY]) {
    let cas = initial
    mockEvent(portal, electionEvent({}, {voter_certificate_policy: "enabled"}))
    portal.graphql.on("sequent_backend_certificate_authority", ({variables}) => {
        // The details drawer asks for a single certificate by id.
        const id = (variables.where as {id?: {_eq?: string}} | undefined)?.id?._eq
        const rows = id ? cas.filter((ca) => ca.id === id) : cas
        return {
            data: {
                sequent_backend_certificate_authority: rows,
                sequent_backend_certificate_authority_aggregate: {aggregate: {count: rows.length}},
            },
        }
    })
    portal.graphql.on("ImportCertificateAuthority", () => {
        cas = [...cas, certificate(IMPORTED_ID, "Imported Voters CA", {})]
        return {
            data: {
                import_certificate_authority: {
                    inserted_count: 1,
                    skipped_count: 1,
                    errors: ["certificate 3: not a CA certificate"],
                },
            },
        }
    })
    portal.graphql.on("DeleteCertificateAuthority", ({variables}) => {
        const ids = variables.ids as string[]
        cas = cas.filter((ca) => !ids.includes(String(ca.id)))
        return {data: {delete_certificate_authority: {deleted_count: ids.length}}}
    })
    portal.graphql.on("ExportCertificateAuthority", () => ({
        data: {
            export_certificate_authority: {
                document_id: "d0000000-0000-4000-8000-000000000001",
                task_execution: exportTask("IN_PROGRESS"),
            },
        },
    }))
    portal.graphql.on("GetTaskById", () => ({
        data: {sequent_backend_tasks_execution: [exportTask("IN_PROGRESS")]},
    }))
}

function exportTask(status: string): Row {
    return {
        id: TASK_ID,
        name: "Export Certificate Authorities",
        execution_status: status,
        created_at: FIXED_TIME,
        start_at: FIXED_TIME,
        end_at: null,
        logs: [],
        annotations: {},
        labels: {},
        executed_by_user: "synthetic-admin",
        tenant_id: TENANT_ID,
        election_event_id: EVENT_ID,
        type: "EXPORT_CERTIFICATE_AUTHORITIES",
    }
}

function wire(portal: AdminPortal, operation: string) {
    return portal.graphql
        .callsTo(operation)
        .map(({variables, headers}) => ({variables, role: headers["x-hasura-role"]}))
}

// Every row repeats the issuer's name, so rows are told apart by their leading common name.
const row = (page: Page, commonName: string) =>
    page.getByRole("row").filter({hasText: new RegExp(`^${commonName}`)})

// With nothing selected, the collapsed bulk toolbar still holds its own Export button after
// the list's export-all action.
const exportAll = (page: Page) => page.getByRole("button", {name: "Export", exact: true}).first()

test.describe("tab visibility", () => {
    test.use({roles: [...CA_ROLES, "election-event-ivr-tab"]})
    const telephone = {voting_channels: {online: true, telephone: true}}

    test("the Certificates tab needs the voter certificate policy", async ({page, portal}) => {
        mockEvent(portal, electionEvent(telephone, {voter_certificate_policy: "disabled"}))
        await openEvent(page, portal)
        await expect(page.getByRole("tab", {name: "IVR", exact: true})).toBeVisible()
        await expect(page.getByRole("tab", {name: "Certificates", exact: true})).toHaveCount(0)
    })

    test.describe("without the Certificates tab role", () => {
        test.use({roles: [...EVENT_ROLES, "election-event-ivr-tab"]})

        test("the Certificates tab stays hidden", async ({page, portal}) => {
            mockEvent(portal, electionEvent(telephone, {voter_certificate_policy: "enabled"}))
            await openEvent(page, portal)
            await expect(page.getByRole("tab", {name: "IVR", exact: true})).toBeVisible()
            await expect(page.getByRole("tab", {name: "Certificates", exact: true})).toHaveCount(0)
        })
    })
})

test("lists the trusted certificates with their type and expiry and shows their details", async ({
    page,
    portal,
}) => {
    mockCertificates(portal)
    await openEvent(page, portal, "Certificates")
    await expect(page.getByRole("row")).toHaveText(
        [
            "Common Name Type Issuer CN Valid From Expires SHA256 Fingerprint Actions",
            "Sequent Root CA Root Sequent Root CA 1/1/2024 1/1/2034 Valid 3f6a9c1e0b7d2a4c5e8f9a0b…",
            "Council Voters CA Intermediate Sequent Root CA 1/1/2024 2/1/2026 Expiring soon bbbbbbbbbbbbbbbbbbbbbbbb…",
            "Legacy Voters CA Intermediate Sequent Root CA 6/1/2020 12/31/2025 Expired cccccccccccccccccccccccc…",
        ],
        {useInnerText: true}
    )
    await row(page, "Sequent Root CA").getByText("3f6a9c1e0b7d2a4c5e8f9a0b…").hover()
    await expect(page.getByRole("tooltip")).toHaveText(ROOT_FINGERPRINT)

    await row(page, "Council Voters CA").locator(".view-ca-icon").click()
    const details = page
        .getByRole("presentation")
        .filter({hasText: "Certificate Authority Details"})
    await expect(details).toContainText("TypeIntermediate")
    await expect(details).toContainText("SubjectCN=Council Voters CA,O=Council")
    await expect(details).toContainText(`Issuer${ROOT_SUBJECT}`)
    await expect(details).toContainText("Expires2/1/2026Expiring soon")
    await expect(details).toContainText("Serial Number4A:1F:00:9C")
    await expect(details).toContainText(`SHA256 Fingerprint${"b".repeat(64)}`)
    await expect(details).toContainText(PEM.trim())
    await details.getByRole("button", {name: "Close", exact: true}).click()
    await expect(details).toHaveCount(0)
})

test("imports a PEM bundle and reports inserted, skipped and rejected certificates", async ({
    page,
    portal,
}) => {
    mockCertificates(portal)
    await openEvent(page, portal, "Certificates")
    await page.getByRole("button", {name: "Import", exact: true}).click()
    const importButton = page.getByRole("button", {name: "Import", exact: true})
    await expect(importButton).toBeDisabled()
    await page.getByLabel("Drop Input File").setInputFiles({
        name: "council-voters.pem",
        mimeType: "application/x-pem-file",
        buffer: Buffer.from(PEM),
    })
    await expect(page.getByText("File loaded (88 bytes)", {exact: true})).toBeVisible()
    await importButton.click()

    await expect(page.getByText("Imported 1 certificate(s).", {exact: true})).toBeVisible()
    expect(wire(portal, "ImportCertificateAuthority")).toEqual([
        {variables: {electionEventId: EVENT_ID, pemContent: PEM}, role: "ca-write"},
    ])
    await expect(row(page, "Imported Voters CA")).toBeVisible()
    await page.clock.runFor(10_000)
    await expect(
        page.getByText(
            "Import issues: 1 certificate(s) skipped (already present).; certificate 3: not a CA certificate",
            {exact: true}
        )
    ).toBeVisible()
})

test("a failed import keeps the drawer open with the error", async ({page, portal}) => {
    mockCertificates(portal)
    portal.graphql.on("ImportCertificateAuthority", () => ({errors: [{message: "malformed PEM"}]}))
    await openEvent(page, portal, "Certificates")
    await page.getByRole("button", {name: "Import", exact: true}).click()
    await page.getByLabel("Drop Input File").setInputFiles({
        name: "broken.pem",
        mimeType: "application/x-pem-file",
        buffer: Buffer.from(PEM),
    })
    await page.getByRole("button", {name: "Import", exact: true}).click()

    await expect(page.getByText("Import failed: malformed PEM", {exact: true})).toBeVisible()
    expect(wire(portal, "ImportCertificateAuthority")).toEqual([
        {variables: {electionEventId: EVENT_ID, pemContent: PEM}, role: "ca-write"},
    ])
    await expect(page.getByText("File loaded (88 bytes)", {exact: true})).toBeVisible()
    await page.getByRole("button", {name: "Cancel", exact: true}).click()
    await page.getByRole("button", {name: "Import", exact: true}).click()
    await expect(page.getByText("Import Certificate Authorities", {exact: true})).toBeVisible()
    await expect(page.getByText(/^File loaded/)).toHaveCount(0)
})

test("deletes a certificate after confirmation and reports a failed delete", async ({
    page,
    portal,
}) => {
    mockCertificates(portal)
    portal.graphql.once("DeleteCertificateAuthority", () => ({errors: [{message: "in use"}]}))
    await openEvent(page, portal, "Certificates")
    const dialog = page.getByRole("dialog")

    await row(page, "Legacy Voters CA").locator(".delete-ca-icon").click()
    await expect(dialog).toContainText("Are you sure you want to delete this item?")
    await dialog.getByRole("button", {name: "Delete", exact: true}).click()
    await expect(page.getByText("Error deleting certificate.", {exact: true})).toBeVisible()
    await expect(row(page, "Legacy Voters CA")).toBeVisible()

    await row(page, "Legacy Voters CA").locator(".delete-ca-icon").click()
    await dialog.getByRole("button", {name: "Delete", exact: true}).click()
    await expect(row(page, "Legacy Voters CA")).toHaveCount(0)
    expect(wire(portal, "DeleteCertificateAuthority")).toEqual([
        {variables: {ids: [LEGACY_ID], electionEventId: EVENT_ID}, role: "ca-write"},
        {variables: {ids: [LEGACY_ID], electionEventId: EVENT_ID}, role: "ca-write"},
    ])
})

test("deletes the selected certificates in bulk", async ({page, portal}) => {
    mockCertificates(portal)
    await openEvent(page, portal, "Certificates")
    await row(page, "Council Voters CA").getByRole("checkbox").check()
    await row(page, "Legacy Voters CA").getByRole("checkbox").check()
    await page.getByRole("button", {name: "Delete", exact: true}).click()
    const dialog = page.getByRole("dialog")
    await expect(dialog).toContainText("Are you sure you want to delete 2 certificate(s)?")
    await dialog.getByRole("button", {name: "Delete", exact: true}).click()

    await expect(page.getByRole("row")).toHaveCount(2)
    await expect(row(page, "Sequent Root CA")).toBeVisible()
    expect(wire(portal, "DeleteCertificateAuthority")).toEqual([
        {variables: {ids: [COUNCIL_ID, LEGACY_ID], electionEventId: EVENT_ID}, role: "ca-write"},
    ])
    await expect(row(page, "Sequent Root CA").getByRole("checkbox")).not.toBeChecked()
})

test("exports all or the selected certificates as a background task", async ({page, portal}) => {
    mockCertificates(portal)
    await openEvent(page, portal, "Certificates")
    const dialog = page.getByRole("dialog")

    await exportAll(page).click()
    await expect(dialog).toContainText("You are about to export all certificate(s).")
    await dialog.getByRole("button", {name: "Cancel", exact: true}).click()
    await expect(dialog).toHaveCount(0)
    expect(wire(portal, "ExportCertificateAuthority")).toEqual([])

    await exportAll(page).click()
    await dialog.getByRole("button", {name: "Export", exact: true}).click()
    await expect(
        page.getByText("Task: Export Certificate Authorities", {exact: true})
    ).toBeVisible()
    await expect(dialog).toHaveCount(0)

    await row(page, "Sequent Root CA").getByRole("checkbox").check()
    await page.getByRole("button", {name: "Export", exact: true}).click()
    await expect(dialog).toContainText("You are about to export 1 certificate(s).")
    await dialog.getByRole("button", {name: "Export", exact: true}).click()
    await expect(dialog).toHaveCount(0)

    expect(wire(portal, "ExportCertificateAuthority")).toEqual([
        {variables: {ids: [], electionEventId: EVENT_ID}, role: "ca-read"},
        {variables: {ids: [ROOT_ID], electionEventId: EVENT_ID}, role: "ca-read"},
    ])
    expect(portal.graphql.callsTo("GetTaskById").map((call) => call.variables)).toContainEqual({
        task_id: TASK_ID,
    })
})

test("a rejected export closes the dialog without following a task", async ({page, portal}) => {
    mockCertificates(portal)
    portal.graphql.on("ExportCertificateAuthority", () => ({errors: [{message: "denied"}]}))
    await openEvent(page, portal, "Certificates")
    await row(page, "Council Voters CA").getByRole("checkbox").check()
    await page.getByRole("button", {name: "Export", exact: true}).click()
    await page.getByRole("dialog").getByRole("button", {name: "Export", exact: true}).click()

    await expect(
        page.getByText("Task: Export Certificate Authorities", {exact: true})
    ).toBeVisible()
    await expect(page.getByRole("dialog")).toHaveCount(0)
    expect(wire(portal, "ExportCertificateAuthority")).toEqual([
        {variables: {ids: [COUNCIL_ID], electionEventId: EVENT_ID}, role: "ca-read"},
    ])
    expect(portal.graphql.callsTo("GetTaskById")).toEqual([])
})

// Widget only seeds its status chip from the status prop, so updateWidgetFail never shows FAILED.
test("a rejected export marks the task as failed", async ({page, portal}) => {
    mockCertificates(portal)
    let rejectExport = () => {}
    const exportReady = new Promise<void>((resolve) => (rejectExport = resolve))
    portal.graphql.on("ExportCertificateAuthority", async () => {
        await exportReady
        return {errors: [{message: "denied"}]}
    })
    await openEvent(page, portal, "Certificates")
    await exportAll(page).click()
    await page.getByRole("dialog").getByRole("button", {name: "Export", exact: true}).click()
    await expect(
        page.getByText("Task: Export Certificate Authorities", {exact: true})
    ).toBeVisible()
    await expect.poll(() => portal.graphql.callsTo("ExportCertificateAuthority").length).toBe(1)
    await expect(page.getByText("IN_PROGRESS", {exact: true})).toBeVisible()
    rejectExport()
    await expect(page.getByRole("dialog")).toHaveCount(0)

    await expect(page.getByText("FAILED", {exact: true})).toBeVisible()
})

test("an event without certificates offers to import the first ones", async ({page, portal}) => {
    mockCertificates(portal, [])
    await openEvent(page, portal, "Certificates")
    await expect(
        page.getByText("No certificate authorities have been imported for this election event.", {
            exact: true,
        })
    ).toBeVisible()
    await page.getByRole("button", {name: "Import Certificates", exact: true}).click()
    await expect(page.getByText("Import Certificate Authorities", {exact: true})).toBeVisible()
})

test.describe("with read-only access", () => {
    test.use({roles: [...CA_ROLES, "ca-read"]})

    test("offers no import or delete controls", async ({page, portal}) => {
        mockCertificates(portal)
        await openEvent(page, portal, "Certificates")
        await expect(row(page, "Sequent Root CA")).toBeVisible()
        await expect(page.getByRole("button", {name: "Import", exact: true})).toHaveCount(0)
        await expect(page.locator(".delete-ca-icon")).toHaveCount(0)
        await row(page, "Sequent Root CA").getByRole("checkbox").check()
        await expect(page.getByRole("button", {name: "Export", exact: true})).toBeVisible()
        await expect(page.getByRole("button", {name: "Delete", exact: true})).toHaveCount(0)
    })

    test("an empty list offers no import", async ({page, portal}) => {
        mockCertificates(portal, [])
        await openEvent(page, portal, "Certificates")
        await expect(
            page.getByText(
                "No certificate authorities have been imported for this election event.",
                {
                    exact: true,
                }
            )
        ).toBeVisible()
        await expect(page.getByRole("button", {name: "Import Certificates"})).toHaveCount(0)
    })
})

test.describe("without ca-read", () => {
    test.use({roles: [...CA_ROLES, "ca-write"]})

    // EditElectionEventCAs always renders the export-all action, but the export runs as ca-read.
    test("offers no export", async ({page, portal}) => {
        mockCertificates(portal)
        await openEvent(page, portal, "Certificates")
        await expect(row(page, "Sequent Root CA")).toBeVisible()

        await expect(page.getByRole("button", {name: "Export"})).toHaveCount(0)
    })
})

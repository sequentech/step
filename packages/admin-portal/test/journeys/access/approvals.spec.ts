// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {
    ALICE_ID,
    AREA_ID,
    DOCUMENT_ID,
    PENDING_APPLICATION_ID,
    REJECTED_APPLICATION_ID,
    application,
    expectRole,
    mockApprovals,
    taskExecution,
    taskRow,
    user,
} from "./data"

const APPROVER_ROLES = [
    "admin-user",
    "election-event-read",
    "election-read",
    "election-event-approvals-tab",
    "application-read",
    "application-write",
]

async function openApprovals(page: Page, portal: PortalServices) {
    await page.goto(`${portal.origin}/sequent_backend_election_event/${IDS.event}?lang=en`)
    await expect(page.getByRole("tab", {name: "Approvals", exact: true})).toBeVisible()
    await expect(page.getByRole("cell", {name: "carol-applicant"}).first()).toBeVisible()
}

async function viewApplication(page: Page, applicantRow: RegExp) {
    // The view action is an unnamed icon button, the only button in its row.
    await page.getByRole("row", {name: applicantRow}).getByRole("button").last().click()
    await expect(page.getByText("Approval Request", {exact: true})).toBeVisible()
}

// The approve action is an unnamed icon button in the matching voter's row.
const approveMatch = (page: Page) =>
    page.getByRole("row", {name: /Carol Voter carol@example\.test/}).getByRole("button")

const matchingVoter = user(ALICE_ID, "carol", {
    first_name: "Carol",
    last_name: "Voter",
    email: "carol@example.test",
})

test.describe("application approver", () => {
    test.use({roles: APPROVER_ROLES})

    test("lists pending applications with the applicant's profile fields", async ({
        page,
        portal,
    }) => {
        mockApprovals(portal, [application(PENDING_APPLICATION_ID, "PENDING")])
        await openApprovals(page, portal)
        const row = page.getByRole("row", {name: /carol-applicant/})
        for (const text of ["MANUAL", "PENDING", "Carol", "Voter", "carol@example.test"])
            await expect(row.getByText(text, {exact: true})).toBeVisible()
        await expect(row.getByText("May 1, 1990", {exact: true})).toBeVisible()
        for (const name of ["Import", "Export"])
            await expect(page.getByRole("button", {name, exact: true})).toHaveCount(0)
        const list = portal.graphql.callsTo("sequent_backend_applications")[0].variables
        expect(list).toMatchObject({
            limit: 10,
            offset: 0,
            order_by: {created_at: "desc"},
        })
        expect(JSON.stringify(list.where)).toContain(`"election_event_id":{"_eq":"${IDS.event}"}`)
        expect(JSON.stringify(list.where)).toContain(`"status":{"_ilike":"%pending%"}`)
        expect(portal.graphql.callsTo("getUserProfileAttributes")[0].variables).toEqual({
            tenantId: TENANT_ID,
            electionEventId: IDS.event,
        })
    })

    test("approves an application against the voter found by its search attributes", async ({
        page,
        portal,
    }) => {
        mockApprovals(portal, [application(PENDING_APPLICATION_ID, "PENDING")], [matchingVoter])
        portal.graphql.on("ChangeApplicationStatus", () => ({
            data: {ApplicationChangeStatus: {message: "Approved", error: null}},
        }))
        await openApprovals(page, portal)
        await viewApplication(page, /carol-applicant/)
        const details = page.getByRole("table", {name: "approvals details table"})
        await expect(details.getByRole("row", {name: /First name\s*Carol/})).toBeVisible()
        await expect(details.getByRole("row", {name: /Birth date\s*1990-05-01/})).toBeVisible()
        await expect(page.getByRole("textbox", {name: "First Name"})).toHaveValue("Carol")
        const matches = portal.graphql.callsTo("getUsers").at(-1)!.variables
        expect(matches).toMatchObject({
            tenant_id: TENANT_ID,
            election_event_id: IDS.event,
            first_name: {IsLike: "Carol"},
            last_name: {IsLike: "Voter"},
            email: {IsLike: "carol@example.test"},
        })
        await approveMatch(page).click()
        const dialog = page.getByRole("dialog")
        await expect(
            dialog.getByText(
                "Are you sure you want to approve this voter? This action is not reversible.",
                {exact: true}
            )
        ).toBeVisible()
        expect(portal.graphql.callsTo("ChangeApplicationStatus")).toHaveLength(0)
        await dialog.getByRole("button", {name: "Approve", exact: true}).click()
        await expect(page.getByText("Voter approved", {exact: true})).toBeVisible()
        await expect(page.getByText("Approval Request", {exact: true})).toHaveCount(0)
        expect(
            portal.graphql.callsTo("ChangeApplicationStatus").map((call) => call.variables)
        ).toEqual([
            {
                tenant_id: TENANT_ID,
                id: PENDING_APPLICATION_ID,
                user_id: ALICE_ID,
                area_id: AREA_ID,
                election_event_id: IDS.event,
            },
        ])
        expectRole(portal, "ChangeApplicationStatus", "admin-user")
    })

    test("explains that the matched voter is already approved", async ({page, portal}) => {
        mockApprovals(portal, [application(PENDING_APPLICATION_ID, "PENDING")], [matchingVoter])
        portal.graphql.on("ChangeApplicationStatus", () => ({
            data: {ApplicationChangeStatus: {message: null, error: "Approved_Voter"}},
        }))
        await openApprovals(page, portal)
        await viewApplication(page, /carol-applicant/)
        await approveMatch(page).click()
        await page.getByRole("dialog").getByRole("button", {name: "Approve", exact: true}).click()
        await expect(page.getByText("Voter is already approved.", {exact: true})).toBeVisible()
        await expect(page.getByText("Approval Request", {exact: true})).toBeVisible()
    })

    test("rejects an application only with a reason, and a message for Other", async ({
        page,
        portal,
    }) => {
        mockApprovals(portal, [application(PENDING_APPLICATION_ID, "PENDING")], [matchingVoter])
        portal.graphql.on("ChangeApplicationStatus", () => ({
            data: {ApplicationChangeStatus: {message: "Rejected", error: null}},
        }))
        await openApprovals(page, portal)
        await viewApplication(page, /carol-applicant/)
        await page.getByRole("button", {name: "Reject Application", exact: true}).click()
        const dialog = page.getByRole("dialog")
        await expect(
            dialog.getByText(
                "Are you sure you want to reject this voter? This action is not reversible."
            )
        ).toBeVisible()
        const submit = dialog.getByRole("button", {name: "Reject Application", exact: true})
        const message = dialog.getByRole("textbox", {name: "Write here the disapproval reason"})
        await message.fill("Document expired")
        await submit.click()
        await expect(dialog.getByText("Required", {exact: true})).toBeVisible()
        await dialog.getByRole("combobox", {name: "Rejection Reason"}).click()
        await page.getByRole("option", {name: "Other", exact: true}).click()
        await message.clear()
        await submit.click()
        await expect(
            dialog.getByText("A rejection message is required for the 'Other' option.", {
                exact: true,
            })
        ).toBeVisible()
        expect(portal.graphql.callsTo("ChangeApplicationStatus")).toHaveLength(0)
        await message.fill("Document expired")
        await submit.click()
        await expect(page.getByText("Voter rejected", {exact: true})).toBeVisible()
        await expect(page.getByText("Approval Request", {exact: true})).toHaveCount(0)
        expect(
            portal.graphql.callsTo("ChangeApplicationStatus").map((call) => call.variables)
        ).toEqual([
            {
                tenant_id: TENANT_ID,
                id: PENDING_APPLICATION_ID,
                user_id: "",
                area_id: AREA_ID,
                election_event_id: IDS.event,
                rejection_reason: "other",
                rejection_message: "Document expired",
            },
        ])
    })

    test("shows why a rejected application was rejected, without a reject action", async ({
        page,
        portal,
    }) => {
        mockApprovals(
            portal,
            [
                application(REJECTED_APPLICATION_ID, "REJECTED", {
                    applicant_id: "carol-applicant-2",
                    annotations: {
                        "search-attributes": "email",
                        "rejection_reason": "no-matching-voter",
                        "verified_by": "synthetic-admin",
                    },
                }),
            ],
            [matchingVoter]
        )
        await page.goto(`${portal.origin}/sequent_backend_election_event/${IDS.event}?lang=en`)
        await page.getByRole("button", {name: "Add filter", exact: true}).click()
        await page.getByRole("menuitemcheckbox", {name: "Status", exact: true}).click()
        await page.getByRole("combobox", {name: "Status"}).click()
        await page.getByRole("option", {name: "Rejected", exact: true}).click()
        await expect(page.getByRole("cell", {name: "carol-applicant-2"})).toBeVisible()
        await expect(page.getByRole("cell", {name: "synthetic-admin"})).toBeVisible()
        await viewApplication(page, /carol-applicant-2/)
        await expect(
            page.getByRole("row", {name: /Rejection Reason\s*No Matching Voter/})
        ).toBeVisible()
        await expect(page.getByRole("button", {name: "Reject Application"})).toHaveCount(0)
        await expect(approveMatch(page)).toBeVisible()
        await page.getByRole("button", {name: "Back", exact: true}).click()
        await expect(page.getByText("Approval Request", {exact: true})).toHaveCount(0)
        const lists = portal.graphql.callsTo("sequent_backend_applications")
        expect(JSON.stringify(lists.at(-1)!.variables.where)).toContain(
            `"status":{"_ilike":"%rejected%"}`
        )
    })
})

test.describe("application transfer operator", () => {
    test.use({roles: [...APPROVER_ROLES, "application-export", "application-import"]})

    test("exports the event's applications with the export role", async ({page, portal}) => {
        mockApprovals(portal, [application(PENDING_APPLICATION_ID, "PENDING")])
        const url = portal.s3.presign("documents/applications.csv", "applications-export")
        portal.graphql.on("ExportApplication", () => ({
            data: {
                export_application: {
                    error_msg: null,
                    document_id: DOCUMENT_ID,
                    task_execution: taskExecution("EXPORT_APPLICATION"),
                },
            },
        }))
        portal.graphql.on("GetTaskById", () => ({
            data: {sequent_backend_tasks_execution: [taskRow("EXPORT_APPLICATION", "SUCCESS")]},
        }))
        portal.graphql.on("GetDocument", () => ({
            data: {sequent_backend_document: [{name: "applications.csv", annotations: {}}]},
        }))
        portal.graphql.on("FetchDocument", () => ({data: {fetchDocument: {url}}}))
        await openApprovals(page, portal)
        await page.getByRole("button", {name: "Export", exact: true}).click()
        const dialog = page.getByRole("dialog")
        await expect(dialog.getByText("Export applications", {exact: true})).toBeVisible()
        const downloading = page.waitForEvent("download")
        await dialog.getByRole("button", {name: "Export", exact: true}).click()
        const download = await downloading
        expect(download.url()).toBe(url)
        expect(download.suggestedFilename()).toBe("export-applications.csv")
        await expect(
            page.getByText("Applications export finished successfully", {exact: true})
        ).toBeVisible()
        expect(portal.graphql.callsTo("ExportApplication").map((call) => call.variables)).toEqual([
            {tenantId: TENANT_ID, electionEventId: IDS.event},
        ])
        expectRole(portal, "ExportApplication", "application-export")
    })

    test("imports applications from a CSV with the import role", async ({page, portal}) => {
        mockApprovals(portal, [application(PENDING_APPLICATION_ID, "PENDING")])
        const csv = Buffer.from("first_name,last_name,email\nDan,Voter,dan@example.test\n")
        const url = portal.s3.presign("documents/applications-upload", "applications-import")
        portal.s3.override(
            (request) =>
                request.method === "PUT" &&
                request.query["X-Amz-Signature"] === "applications-import",
            {status: 200},
            1
        )
        portal.graphql.on("GetUploadUrl", () => ({
            data: {get_upload_url: {url, document_id: DOCUMENT_ID}},
        }))
        portal.graphql.on("ImportApplication", () => ({
            data: {
                import_application: {
                    error_msg: null,
                    document_id: DOCUMENT_ID,
                    task_execution: taskExecution("IMPORT_APPLICATION"),
                },
            },
        }))
        portal.graphql.on("GetTaskById", () => ({
            data: {sequent_backend_tasks_execution: [taskRow("IMPORT_APPLICATION", "SUCCESS")]},
        }))
        await openApprovals(page, portal)
        await page.getByRole("button", {name: "Import", exact: true}).click()
        const drawer = page.getByRole("dialog")
        await expect(drawer.getByText("Import applications", {exact: true})).toBeVisible()
        await drawer.getByRole("textbox", {name: "Integrity Check (SHA-256)"}).fill("c".repeat(64))
        const upload = page.waitForRequest(
            (request) => request.url() === url && request.method() === "PUT"
        )
        await drawer.locator('input[type="file"]').setInputFiles({
            name: "applications.csv",
            mimeType: "text/csv",
            buffer: csv,
        })
        expect((await upload).postDataBuffer()).toEqual(csv)
        await drawer.getByRole("button", {name: "Import", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("ImportApplication").length).toBe(1)
        expect(portal.graphql.callsTo("ImportApplication")[0].variables).toEqual({
            tenantId: TENANT_ID,
            electionEventId: IDS.event,
            documentId: DOCUMENT_ID,
            sha256: "c".repeat(64),
        })
        await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
        expectRole(portal, "ImportApplication", "application-import")
    })
})

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
    // Authentication can redirect before the event has loaded. Wait for the
    // event read that supplies the tab's record before checking its permission.
    const eventLoaded = page.waitForResponse(
        (response) =>
            response.url() === `${portal.origin}/v1/graphql` &&
            response.request().postDataJSON()?.operationName === "sequent_backend_election_event"
    )
    await page.goto(`${portal.origin}/sequent_backend_election_event/${IDS.event}?lang=en`)
    await eventLoaded
    await expect(page.getByRole("tab", {name: "Approvals", exact: true})).toBeVisible()
    await expect(page.getByRole("cell", {name: /Carol Voter/}).first()).toBeVisible()
}

const backToList = (page: Page) =>
    page.getByRole("button", {name: "Back to Approvals", exact: true})

/** Opens the review of the only enrollment in the queue through its row actions. */
async function reviewEnrollment(page: Page, action = "Review enrollment") {
    await page.getByRole("button", {name: "Actions", exact: true}).click()
    await page.getByRole("menuitem", {name: action, exact: true}).click()
    await expect(page.getByRole("heading", {name: "Carol Voter", exact: true})).toBeVisible()
    await expect(backToList(page)).toBeVisible()
}

const nextStep = (page: Page) => page.getByRole("button", {name: "Continue", exact: true}).click()

const registry = (page: Page) => page.getByRole("radiogroup", {name: "Voters in the registry"})

/** Steps 1 and 2 of the review: past the identity check, choosing `voter` in the registry. */
async function chooseVoter(page: Page, voter: RegExp) {
    await nextStep(page)
    await registry(page).getByRole("radio", {name: voter}).check()
    await nextStep(page)
}

const matchingVoter = user(ALICE_ID, "carol", {
    first_name: "Carol",
    last_name: "Voter",
    email: "carol@example.test",
})

test.describe("application approver", () => {
    test.use({roles: APPROVER_ROLES})

    test("lists the enrollments that wait for a person, with what happened to each", async ({
        page,
        portal,
    }) => {
        mockApprovals(portal, [application(PENDING_APPLICATION_ID, "PENDING")])
        await openApprovals(page, portal)
        const row = page.getByRole("row", {name: /Carol Voter/})
        for (const text of [
            "Carol Voter",
            "carol@example.test",
            "Waiting for a person to decide",
            "Applied Jan 15, 2026",
            "Needs review",
        ])
            await expect(row.getByText(text, {exact: true})).toBeVisible()
        await expect(page.getByRole("combobox", {name: "Status"})).toHaveText("Needs review")
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

    test("approves an enrollment for the registry voter found by its compared details", async ({
        page,
        portal,
    }) => {
        mockApprovals(portal, [application(PENDING_APPLICATION_ID, "PENDING")], [matchingVoter])
        portal.graphql.on("ChangeApplicationStatus", () => ({
            data: {ApplicationChangeStatus: {message: "Approved", error: null}},
        }))
        await openApprovals(page, portal)
        await reviewEnrollment(page)
        await expect(page.getByText("Why this needs a person", {exact: true})).toBeVisible()
        await expect(page.getByRole("row", {name: /First name\s*Carol/})).toBeVisible()
        await expect(page.getByRole("row", {name: /Birth date\s*May 1, 1990/})).toBeVisible()
        await expect(
            page.getByRole("row", {name: new RegExp(`Application ID\\s*${PENDING_APPLICATION_ID}`)})
        ).toBeVisible()

        await nextStep(page)
        const voter = registry(page).getByRole("radio", {name: /Carol Voter/})
        await expect(registry(page).getByText("Best match · 3 of 3 details match")).toBeVisible()
        for (const detail of ["First name", "Last name", "Email"])
            await expect(
                page.getByRole("row", {name: new RegExp(`^${detail}\\b.*Same$`)})
            ).toBeVisible()
        // One lookup has every compared detail; the others leave one out each.
        const lookups = portal.graphql.callsTo("getUsers").map((call) => call.variables)
        expect(lookups).toContainEqual(
            expect.objectContaining({
                tenant_id: TENANT_ID,
                election_event_id: IDS.event,
                first_name: {IsLike: "Carol"},
                last_name: {IsLike: "Voter"},
                email: {IsLike: "carol@example.test"},
            })
        )

        // Approving waits for the voter to be chosen.
        await nextStep(page)
        const send = page.getByRole("button", {name: "Approve enrollment", exact: true})
        await expect(send).toBeDisabled()
        await expect(
            page.getByText("Choose the matching voter in step 2 to approve.", {exact: true})
        ).toBeVisible()
        await page.getByRole("button", {name: /Find the voter/}).click()
        await voter.check()
        await nextStep(page)
        await send.click()
        const dialog = page.getByRole("dialog")
        await expect(dialog.getByText("Approve Carol Voter?", {exact: true})).toBeVisible()
        await expect(dialog.getByText("3 of 3 details match", {exact: true})).toBeVisible()
        await expect(dialog.getByText("This can't be undone.", {exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("ChangeApplicationStatus")).toHaveLength(0)
        await dialog.getByRole("button", {name: "Approve", exact: true}).click()
        await expect(
            page.getByText("Carol Voter approved. The voter has been told.", {exact: true})
        ).toBeVisible()
        await expect(backToList(page)).toHaveCount(0)
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

    test("explains that the chosen voter is already enrolled", async ({page, portal}) => {
        mockApprovals(portal, [application(PENDING_APPLICATION_ID, "PENDING")], [matchingVoter])
        portal.graphql.on("ChangeApplicationStatus", () => ({
            data: {ApplicationChangeStatus: {message: null, error: "Approved_Voter"}},
        }))
        await openApprovals(page, portal)
        await reviewEnrollment(page)
        await chooseVoter(page, /Carol Voter/)
        await page.getByRole("button", {name: "Approve enrollment", exact: true}).click()
        await page.getByRole("dialog").getByRole("button", {name: "Approve", exact: true}).click()
        await expect(page.getByText("This voter is already enrolled.", {exact: true})).toBeVisible()
        await expect(backToList(page)).toBeVisible()
    })

    test("rejects an enrollment without a matching voter, with a message for Other", async ({
        page,
        portal,
    }) => {
        mockApprovals(portal, [application(PENDING_APPLICATION_ID, "PENDING")], [matchingVoter])
        portal.graphql.on("ChangeApplicationStatus", () => ({
            data: {ApplicationChangeStatus: {message: "Rejected", error: null}},
        }))
        await openApprovals(page, portal)
        await reviewEnrollment(page)
        await chooseVoter(page, /None of these is the voter/)
        await expect(
            page.getByText(
                "You found no matching voter, so this enrollment can only be rejected.",
                {
                    exact: true,
                }
            )
        ).toBeVisible()
        const verdict = page.getByRole("radiogroup", {name: "Decide", exact: true})
        await expect(verdict.getByRole("radio", {name: /Approve/})).toBeDisabled()
        await expect(verdict.getByRole("radio", {name: /Reject/})).toBeChecked()
        const reasons = page.getByRole("radiogroup", {name: "Reason for rejecting"})
        await expect(reasons.getByRole("radio", {name: /No matching voter/})).toBeChecked()
        await expect(
            page.getByText(/We couldn't find a voter in the registry that matches your details/)
        ).toBeVisible()
        await reasons.getByRole("radio", {name: /Other/}).check()
        const submit = page.getByRole("button", {name: "Reject enrollment", exact: true})
        await expect(
            page.getByText("Write a message for the voter when the reason is Other.", {exact: true})
        ).toBeVisible()
        await expect(submit).toBeDisabled()
        expect(portal.graphql.callsTo("ChangeApplicationStatus")).toHaveLength(0)
        await page.getByRole("textbox", {name: "Message to the voter"}).fill("Document expired")
        await submit.click()
        await expect(
            page.getByText("Carol Voter rejected. The voter has been told.", {exact: true})
        ).toBeVisible()
        await expect(backToList(page)).toHaveCount(0)
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

    test("shows who rejected an enrollment and why, without a reject action", async ({
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
        await page.getByRole("combobox", {name: "Status"}).click()
        await page.getByRole("option", {name: "Rejected", exact: true}).click()
        const row = page.getByRole("row", {name: /Carol Voter/})
        await expect(row.getByText("Rejected by synthetic-admin", {exact: true})).toBeVisible()
        await reviewEnrollment(page, "Open enrollment")
        await expect(page.getByText("How this was decided", {exact: true})).toBeVisible()
        await expect(
            page.getByText(
                "synthetic-admin rejected this enrollment on Jan 15, 2026: no matching voter.",
                {exact: true}
            )
        ).toBeVisible()
        // A rejected enrollment can still be approved for the right voter, not rejected again.
        await chooseVoter(page, /Carol Voter/)
        const verdict = page.getByRole("radiogroup", {name: "Decide", exact: true})
        await expect(verdict.getByRole("radio", {name: /Approve/})).toBeChecked()
        await expect(verdict.getByRole("radio", {name: /Reject/})).toHaveCount(0)
        await expect(page.getByRole("button", {name: "Reject enrollment"})).toHaveCount(0)
        await backToList(page).click()
        await expect(backToList(page)).toHaveCount(0)
        await expect(row).toBeVisible()
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

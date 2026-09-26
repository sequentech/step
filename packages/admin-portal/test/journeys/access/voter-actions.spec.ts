// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {
    ALICE_ID,
    BOB_ID,
    DOCUMENT_ID,
    TASK_ID,
    VOTER_TAB_ROLES,
    attribute,
    expectRole,
    mockVoters,
    openVoters,
    rowAction,
    taskExecution,
    taskRow,
    user,
} from "./data"

const ROW_ACTION_ROLES = [
    ...VOTER_TAB_ROLES,
    "voter-write",
    "voter-delete",
    "voter-manually-verify",
    "voter-change-password",
    "notification-send",
    "ee-voters-logs",
    "voter-information-letter",
    "document-password-read",
]

test.describe("voter operator with every row action", () => {
    test.use({roles: ROW_ACTION_ROLES})

    test("lists every row action the token grants", async ({page, portal}) => {
        mockVoters(portal)
        await openVoters(page, portal)
        await page.getByRole("button", {name: "Actions", exact: true}).click()
        await expect(page.getByRole("menuitem")).toHaveText([
            "Send",
            "Edit",
            "Delete",
            "Manually Verify",
            "Voter Information Letter",
            "Change password",
            "User's Logs",
        ])
    })

    test("changes a voter password as temporary after a policy rejection", async ({
        page,
        portal,
    }) => {
        mockVoters(portal)
        portal.graphql.once("EditUser", () => ({
            errors: [
                {
                    message: "Password policy violation",
                    extensions: {
                        code: "PasswordPolicyViolation",
                        password_policy_rule: "minimumLength",
                        password_policy_required_count: 12,
                    },
                },
            ],
        }))
        portal.graphql.on("EditUser", () => ({
            data: {edit_user: {user: {id: ALICE_ID}, task_execution: null}},
        }))
        await openVoters(page, portal)
        await rowAction(page, "Change password")
        const dialog = page.getByRole("dialog")
        await expect(dialog.getByText("Change password", {exact: true})).toBeVisible()
        // The password inputs carry no accessible name; their form field names are the contract.
        const password = dialog.locator('input[name="password"]')
        const confirm = dialog.locator('input[name="confirm_password"]')
        await password.fill("short")
        await confirm.fill("shorter")
        await expect(dialog.getByText("Passwords must match", {exact: true})).toBeVisible()
        await confirm.fill("short")
        await dialog.getByRole("button", {name: "Save", exact: true}).click()
        await expect(
            dialog.getByText("The password minimum length is 12.", {exact: true})
        ).toBeVisible()
        await password.fill("a-long-enough-secret")
        await confirm.fill("a-long-enough-secret")
        await dialog.getByRole("button", {name: "Save", exact: true}).click()
        await expect(dialog).toHaveCount(0)
        await expect(page.getByText("Voter edited", {exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("EditUser").map(({variables}) => variables)).toEqual([
            {
                body: {
                    user_id: ALICE_ID,
                    tenant_id: TENANT_ID,
                    election_event_id: IDS.event,
                    password: "short",
                    temporary: true,
                },
            },
            {
                body: {
                    user_id: ALICE_ID,
                    tenant_id: TENANT_ID,
                    election_event_id: IDS.event,
                    password: "a-long-enough-secret",
                    temporary: true,
                },
            },
        ])
        expectRole(portal, "EditUser", "admin-user")
    })

    test("manually verifies a reachable voter and downloads the QR letter", async ({
        page,
        portal,
    }) => {
        mockVoters(portal, {
            users: [user(ALICE_ID, "alice"), user(BOB_ID, "bob", {email: null})],
        })
        const url = portal.s3.presign("documents/manual-verify.pdf", "manual-verify")
        portal.graphql.on("ManualVerification", () => ({
            data: {get_manual_verification_pdf: {document_id: DOCUMENT_ID, status: "ok"}},
        }))
        portal.graphql.on("GetDocument", () => ({
            data: {sequent_backend_document: [{name: "manual-verify.pdf", annotations: {}}]},
        }))
        portal.graphql.on("FetchDocument", () => ({
            data: {fetchDocument: {url}},
        }))
        await openVoters(page, portal)
        const dialog = page.getByRole("dialog")
        await page
            .getByRole("row", {name: /bob/})
            .getByRole("button", {name: "Actions", exact: true})
            .click()
        await page.getByRole("menuitem", {name: "Manually Verify", exact: true}).click()
        await expect(
            dialog.getByText(
                "This voter can not be manually verified because they do not have an email address or phone number attributed to them.",
                {exact: true}
            )
        ).toBeVisible()
        await expect(
            dialog.getByRole("button", {name: "Manually Verify this voter", exact: true})
        ).toBeDisabled()
        await dialog.getByRole("button", {name: "Cancel", exact: true}).click()
        await expect(dialog).toHaveCount(0)

        await page
            .getByRole("row", {name: /alice/})
            .getByRole("button", {name: "Actions", exact: true})
            .click()
        await page.getByRole("menuitem", {name: "Manually Verify", exact: true}).click()
        const downloading = page.waitForEvent("download")
        await dialog.getByRole("button", {name: "Manually Verify this voter", exact: true}).click()
        const download = await downloading
        expect(download.url()).toBe(url)
        expect(download.suggestedFilename()).toBe(`manual-verify-${IDS.event}-${DOCUMENT_ID}.pdf`)
        await expect(
            page.getByText("Sucessfully verified manually the voter, downloading PDF..", {
                exact: true,
            })
        ).toBeVisible()
        await expect(dialog).toHaveCount(0)
        expect(portal.graphql.callsTo("ManualVerification").map((call) => call.variables)).toEqual([
            {tenantId: TENANT_ID, electionEventId: IDS.event, voterId: ALICE_ID},
        ])
        expect(portal.graphql.callsTo("FetchDocument")[0].variables).toEqual({
            electionEventId: IDS.event,
            documentId: DOCUMENT_ID,
        })
        expect(portal.graphql.callsTo("GetDocument")[0].variables).toEqual({
            id: DOCUMENT_ID,
            tenantId: TENANT_ID,
        })
        expectRole(portal, "ManualVerification", "admin-user")
    })

    test("generates a voter information letter and reveals its one-time PDF password", async ({
        page,
        portal,
    }) => {
        mockVoters(portal)
        portal.graphql.on("GenerateVoterInformationLetter", () => ({
            data: {
                generate_voter_information_letter: {
                    document_id: DOCUMENT_ID,
                    pdf_password: "Synthetic-Pdf-Pass-42",
                    task_execution: taskExecution("VOTER_INFORMATION_LETTER"),
                },
            },
        }))
        portal.graphql.on("GetTaskById", () => ({
            data: {
                sequent_backend_tasks_execution: [
                    taskRow("VOTER_INFORMATION_LETTER", "IN_PROGRESS"),
                ],
            },
        }))
        await openVoters(page, portal)
        await rowAction(page, "Voter Information Letter")
        const dialog = page.getByRole("dialog")
        await expect(
            dialog.getByText(
                "Generate a Voter Information Letter for this voter? A new password will be assigned and included in an encrypted PDF.",
                {exact: true}
            )
        ).toBeVisible()
        expect(portal.graphql.callsTo("GenerateVoterInformationLetter")).toHaveLength(0)
        await dialog.getByRole("button", {name: "Generate", exact: true}).click()
        await expect(
            page.getByText("Voter Information Letter generation started", {exact: true})
        ).toBeVisible()
        await expect(
            page.getByRole("textbox", {name: "Password to open the encrypted PDF"})
        ).toHaveValue("Synthetic-Pdf-Pass-42")
        await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
        expect(portal.graphql.callsTo("GetTaskById")[0].variables).toEqual({task_id: TASK_ID})
        expect(
            portal.graphql.callsTo("GenerateVoterInformationLetter").map((call) => call.variables)
        ).toEqual([{electionEventId: IDS.event, voterId: ALICE_ID}])
        expectRole(portal, "GenerateVoterInformationLetter", "voter-information-letter")
    })

    test("explains a letter refused for a missing password policy", async ({page, portal}) => {
        mockVoters(portal)
        portal.graphql.on("GenerateVoterInformationLetter", () => ({
            errors: [
                {
                    message: "Password Policy is not configured.",
                    extensions: {code: "PasswordPolicyNotConfigured"},
                },
            ],
        }))
        await openVoters(page, portal)
        await rowAction(page, "Voter Information Letter")
        await page.getByRole("dialog").getByRole("button", {name: "Generate", exact: true}).click()
        await expect(
            page.getByText(
                "Password Policy is not configured. Set it under Election Event Data before generating a letter.",
                {exact: true}
            )
        ).toBeVisible()
        await expect(page.getByRole("dialog")).toHaveCount(0)
        await expect(
            page.getByRole("textbox", {name: "Password to open the encrypted PDF"})
        ).toHaveCount(0)
    })

    test("tells the operator when a voter deletion is refused", async ({page, portal}) => {
        mockVoters(portal)
        portal.graphql.on("DeleteUser", () => ({
            errors: [
                {message: "Synthetic voter deletion refused", extensions: {code: "Unauthorized"}},
            ],
        }))
        // Today the refusal escapes as an unhandled rejection; tolerate exactly that one so the
        // missing notification below is what fails.
        await page.addInitScript(() =>
            window.addEventListener("unhandledrejection", (event) => {
                if (String(event.reason?.message) === "Synthetic voter deletion refused")
                    event.preventDefault()
            })
        )
        await openVoters(page, portal)
        await rowAction(page, "Delete")
        await page.getByRole("dialog").getByRole("button", {name: "Delete", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("DeleteUser").length).toBe(1)
        expect(portal.graphql.callsTo("DeleteUser")[0].variables).toEqual({
            tenantId: TENANT_ID,
            electionEventId: IDS.event,
            userId: ALICE_ID,
        })
        expectRole(portal, "DeleteUser", "admin-user")
        test.fail(true, "ListUsers awaits DeleteUser without catching Apollo's rejection")
        await expect(page.getByText("Error deleting voter", {exact: true})).toBeVisible({
            timeout: 2000,
        })
    })
})

test.describe("letter permission without the document password permission", () => {
    test.use({roles: [...VOTER_TAB_ROLES, "voter-information-letter"]})

    test("does not offer the voter information letter", async ({page, portal}) => {
        mockVoters(portal)
        await openVoters(page, portal)
        await expect(page.getByRole("cell", {name: "alice", exact: true})).toBeVisible()
        await expect(page.getByRole("button", {name: "Actions", exact: true})).toHaveCount(0)
    })
})

test.describe("password-only voter operator", () => {
    test.use({roles: [...VOTER_TAB_ROLES, "voter-change-password"]})

    test("offers only the password change", async ({page, portal}) => {
        mockVoters(portal)
        await openVoters(page, portal)
        await page.getByRole("button", {name: "Actions", exact: true}).click()
        await expect(page.getByRole("menuitem")).toHaveText(["Change password"])
        for (const name of ["Add", "Import", "Export", "Send"])
            await expect(page.getByRole("button", {name, exact: true})).toHaveCount(0)
    })
})

const LIST_MANAGER_ROLES = [
    ...VOTER_TAB_ROLES,
    "voter-import",
    "voter-export",
    "voter-delete",
    "voter-secret-attribute-read",
    "notification-send",
    "communication-template-read",
]
const CSV = Buffer.from("username,email,area\ncarol,carol@example.test,North\n")
const CHECKSUM = "b".repeat(64)

test.describe("voter list manager", () => {
    test.use({roles: LIST_MANAGER_ROLES})

    test("exports voters with decrypted secret fields only after opting in", async ({
        page,
        portal,
    }) => {
        mockVoters(portal, {
            attributes: [
                attribute("username", {display_name: "Username"}),
                attribute("reference", {annotations: {"sequent.secret": "true"}}),
            ],
        })
        const url = portal.s3.presign("documents/voters.csv", "voters-export")
        portal.graphql.on("ExportUsers", () => ({
            data: {
                export_users: {
                    error_msg: null,
                    document_id: DOCUMENT_ID,
                    task_execution: taskExecution("EXPORT_VOTERS"),
                },
            },
        }))
        portal.graphql.on("GetTaskById", () => ({
            data: {
                sequent_backend_tasks_execution: [
                    taskRow("EXPORT_VOTERS", "SUCCESS", {document_id: DOCUMENT_ID}),
                ],
            },
        }))
        portal.graphql.on("GetDocument", () => ({
            data: {sequent_backend_document: [{name: "voters.csv", annotations: {}}]},
        }))
        portal.graphql.on("FetchDocument", () => ({data: {fetchDocument: {url}}}))
        await openVoters(page, portal)
        await page.getByRole("button", {name: "Export", exact: true}).click()
        const dialog = page.getByRole("dialog")
        const include = dialog.getByRole("checkbox", {
            name: "Include decrypted secret voter fields",
        })
        await expect(include).not.toBeChecked()
        const warning = dialog.getByText(
            "Sensitive export: the downloaded CSV will contain these fields in plaintext.",
            {exact: true}
        )
        await expect(warning).toHaveCount(0)
        await include.check()
        await expect(warning).toBeVisible()
        const downloading = page.waitForEvent("download")
        await dialog.getByRole("button", {name: "Export", exact: true}).click()
        expect((await downloading).url()).toBe(url)
        await expect(dialog).toHaveCount(0)
        expect(portal.graphql.callsTo("ExportUsers").map((call) => call.variables)).toEqual([
            {tenantId: TENANT_ID, electionEventId: IDS.event, includeSecretAttributes: true},
        ])
        expect(portal.graphql.callsTo("GetTaskById")[0].variables).toEqual({task_id: TASK_ID})
        expectRole(portal, "ExportUsers", "admin-user")
    })

    test("deletes the selected voters or every voter matching the active filters", async ({
        page,
        portal,
    }) => {
        mockVoters(portal, {users: [user(ALICE_ID, "alice"), user(BOB_ID, "bob")], total: 40})
        portal.graphql.on("DeleteUsers", () => ({
            data: {
                delete_users: {
                    ids: null,
                    error_msg: null,
                    task_execution: taskExecution("DELETE_VOTERS"),
                },
            },
        }))
        portal.graphql.on("GetTaskById", () => ({
            data: {sequent_backend_tasks_execution: [taskRow("DELETE_VOTERS", "SUCCESS")]},
        }))
        await openVoters(page, portal)
        const dialog = page.getByRole("dialog")
        await page.getByRole("row", {name: /alice/}).getByRole("checkbox").check()
        await page.getByRole("button", {name: "Delete", exact: true}).click()
        await expect(
            dialog.getByText(
                "1 voters are selected. You can instead delete every voter matching the current filters, which may be more. This cannot be undone.",
                {exact: true}
            )
        ).toBeVisible()
        expect(portal.graphql.callsTo("DeleteUsers")).toHaveLength(0)
        await dialog.getByRole("button", {name: "Delete 1 selected", exact: true}).click()
        await expect(page.getByText("Voters deleted", {exact: true})).toBeVisible()
        await page.getByRole("row", {name: /bob/}).getByRole("checkbox").check()
        await page.getByRole("button", {name: "Delete", exact: true}).click()
        await dialog.getByRole("button", {name: "Delete all matching", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("DeleteUsers").length).toBe(2)
        expect(portal.graphql.callsTo("DeleteUsers").map((call) => call.variables)).toEqual([
            {
                tenantId: TENANT_ID,
                electionEventId: IDS.event,
                usersId: [ALICE_ID],
                selectAll: false,
            },
            {
                tenantId: TENANT_ID,
                electionEventId: IDS.event,
                selectAll: true,
                email: null,
                username: null,
                first_name: null,
                last_name: null,
                attributes: null,
                enabled: null,
                email_verified: null,
                has_voted: null,
                authorized_to_election_alias: null,
            },
        ])
        expectRole(portal, "DeleteUsers", "admin-user")
    })

    test("imports a voter CSV with its checksum as a tracked task", async ({page, portal}) => {
        mockVoters(portal)
        const url = portal.s3.presign("documents/voters-upload", "voters-import")
        portal.s3.override(
            (request) =>
                request.method === "PUT" && request.query["X-Amz-Signature"] === "voters-import",
            {status: 200},
            1
        )
        portal.graphql.on("GetUploadUrl", () => ({
            data: {get_upload_url: {url, document_id: DOCUMENT_ID}},
        }))
        portal.graphql.on("ImportUsers", () => ({
            data: {import_users: {task_execution: taskExecution("IMPORT_USERS")}},
        }))
        portal.graphql.on("GetTaskById", () => ({
            data: {sequent_backend_tasks_execution: [taskRow("IMPORT_USERS", "SUCCESS")]},
        }))
        await openVoters(page, portal)
        await page.getByRole("button", {name: "Import", exact: true}).click()
        const drawer = page.getByRole("dialog")
        await expect(drawer.getByText("Import Voters", {exact: true})).toBeVisible()
        await drawer.getByRole("textbox", {name: "Integrity Check (SHA-256)"}).fill(CHECKSUM)
        const upload = page.waitForRequest(
            (request) => request.url() === url && request.method() === "PUT"
        )
        await drawer.locator('input[type="file"]').setInputFiles({
            name: "voters.csv",
            mimeType: "text/csv",
            buffer: CSV,
        })
        expect((await upload).postDataBuffer()).toEqual(CSV)
        await expect(
            page.getByText("File uploaded to server - but not imported yet", {exact: true})
        ).toBeVisible()
        await drawer.getByRole("button", {name: "Import", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("ImportUsers").length).toBe(1)
        expect(portal.graphql.callsTo("GetUploadUrl")[0].variables).toEqual({
            name: "voters.csv",
            media_type: "text/csv",
            size: CSV.length,
            is_public: false,
        })
        expect(portal.graphql.callsTo("ImportUsers")[0].variables).toEqual({
            tenantId: TENANT_ID,
            documentId: DOCUMENT_ID,
            electionEventId: IDS.event,
            sha256: CHECKSUM,
        })
        await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
        expectRole(portal, "ImportUsers", "admin-user")
    })

    test("schedules a notification to the selected voter with the secret fields it uses", async ({
        page,
        portal,
    }) => {
        mockVoters(portal, {
            attributes: [
                attribute("username", {display_name: "Username"}),
                attribute("reference", {annotations: {"sequent.secret": "true"}}),
            ],
        })
        portal.graphql.on("sequent_backend_template", () => ({
            data: {
                sequent_backend_template: [],
                sequent_backend_template_aggregate: {aggregate: {count: 0}},
            },
        }))
        portal.graphql.on("CreateScheduledEvent", () => ({
            data: {createScheduledEvent: {id: "77777777-7777-4777-8777-777777777701"}},
        }))
        await openVoters(page, portal)
        await page.getByRole("button", {name: "Actions", exact: true}).click()
        await page.getByRole("menuitem", {name: "Send", exact: true}).click()
        const drawer = page.getByRole("dialog")
        await expect(drawer.getByText("Send Notification", {exact: true}).first()).toBeVisible()
        await expect(drawer.getByText("To 1 Selected voters", {exact: true})).toBeVisible()
        await drawer
            .getByRole("textbox", {name: "Email Subject"})
            .fill('Your code {{lookup user.attributes "reference"}}')
        await drawer.getByRole("button", {name: "Send Notification", exact: true}).click()
        await expect(
            page.getByText("Notification programmed/sent successfully", {exact: true})
        ).toBeVisible()
        await expect(drawer).toHaveCount(0)
        expect(
            portal.graphql.callsTo("CreateScheduledEvent").map((call) => call.variables)
        ).toEqual([
            {
                tenantId: TENANT_ID,
                electionEventId: IDS.event,
                eventProcessor: "SEND_TEMPLATE",
                eventPayload: {
                    audience_selection: "SELECTED",
                    audience_voter_ids: [ALICE_ID],
                    communication_method: "EMAIL",
                    schedule_now: true,
                    email: {
                        subject: 'Your code {{lookup user.attributes "reference"}}',
                        plaintext_body:
                            "Hello {{user.first_name}},\n\nEnter in {{vote_url}} to vote",
                        html_body:
                            "<p>Hello {{user.first_name}},<br><br>Enter in {{vote_url}} to vote</p>",
                    },
                    sms: {message: "Enter in {{vote_url}} to vote"},
                    secret_attribute_names: ["reference"],
                },
            },
        ])
        expectRole(portal, "CreateScheduledEvent", "admin-user")
        expectRole(portal, "sequent_backend_template", "communication-template-read")
    })
})

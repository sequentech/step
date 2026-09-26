// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "./fixtures"

const DOCUMENT_ID = "33333333-3333-4333-8333-333333333333"
const TASK_ID = "44444444-4444-4444-8444-444444444444"
const CHECKSUM = "ab".repeat(32)
const CONTENT = Buffer.from('{"elections":[]}')
function eventWorkflow(portal: PortalServices, kind: "create" | "import", validationError = false) {
    let phase = "IN_PROGRESS"
    let started = false
    let eventId = IDS.event as string
    let eventName = "Imported council"
    const record = () => ({
        id: eventId,
        tenant_id: TENANT_ID,
        name: eventName,
        description: "Annual council election",
        encryption_protocol: "RSA256",
        is_archived: false,
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        elections: [],
        elections_aggregate: {aggregate: {count: 0}, nodes: []},
        presentation: {
            i18n: {en: {name: eventName}},
            language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
        },
        status: {},
        voting_channels: {online: true},
    })
    const task = () => ({
        id: TASK_ID,
        tenant_id: TENANT_ID,
        election_event_id: eventId,
        name: "Synthetic event workflow",
        type: kind === "create" ? "CREATE_ELECTION_EVENT" : "IMPORT_ELECTION_EVENT",
        execution_status: phase,
        created_at: FIXED_TIME,
        start_at: FIXED_TIME,
        end_at: phase === "IN_PROGRESS" ? null : FIXED_TIME,
        executed_by_user: IDS.voter,
        annotations: {},
        labels: {},
        logs: [
            {
                created_date: FIXED_TIME,
                log_text:
                    phase === "FAILED"
                        ? "Synthetic event task failed"
                        : "Synthetic event task running",
            },
        ],
    })
    portal.settings.QUERY_POLL_INTERVAL_MS = 100
    portal.settings.QUERY_FAST_POLL_INTERVAL_MS = 100
    portal.graphql.on("sequent_backend_election_event", () => ({
        data: {
            sequent_backend_election_event: started && phase === "SUCCESS" ? [record()] : [],
            sequent_backend_election_event_aggregate: {
                aggregate: {count: started && phase === "SUCCESS" ? 1 : 0},
            },
        },
    }))
    portal.graphql.on("election_events_tree", () => ({
        data: {sequent_backend_election_event: started && phase === "SUCCESS" ? [record()] : []},
    }))
    portal.graphql.on("election_tree", () => ({data: {sequent_backend_election: []}}))
    portal.graphql.on("GetTaskById", () => ({data: {sequent_backend_tasks_execution: [task()]}}))
    portal.graphql.on("CreateElectionEvent", ({variables}) => {
        const input = variables.electionEvent as Record<string, unknown>
        eventId = String(input.id)
        eventName = String(input.name)
        started = true
        return {
            data: {
                insertElectionEvent: {
                    id: eventId,
                    error: null,
                    message: "Created",
                    task_execution: task(),
                },
            },
        }
    })
    portal.graphql.on("ImportElectionEvent", ({variables}) => {
        if (variables.checkOnly)
            return {
                data: {
                    import_election_event: {
                        id: null,
                        error: validationError ? "Synthetic archive validation failed" : null,
                        message: "Checked",
                        task_execution: null,
                    },
                },
            }
        started = true
        return {
            data: {
                import_election_event: {
                    id: eventId,
                    error: null,
                    message: "Import started",
                    task_execution: task(),
                },
            },
        }
    })
    const key = "documents/event-upload"
    const url = portal.s3.presign(key, "event-import")
    portal.s3.override(
        (request) =>
            request.method === "PUT" &&
            request.key === key &&
            request.query["X-Amz-Signature"] === "event-import",
        {status: 200},
        1
    )
    portal.graphql.on("GetUploadUrl", () => ({
        data: {get_upload_url: {url, document_id: DOCUMENT_ID}},
    }))
    return {
        url,
        eventId: () => eventId,
        complete: (outcome: "SUCCESS" | "FAILED") => {
            phase = outcome
        },
    }
}
async function boot(page: Page, portal: PortalServices) {
    await page.goto(`${portal.origin}/?lang=en`)
    await expect(page.getByText("No Election Event yet", {exact: true})).toBeVisible()
}
async function selectArchive(
    page: Page,
    portal: PortalServices,
    url: string,
    encrypted: boolean,
    whileUploading?: () => Promise<void>
) {
    await boot(page, portal)
    await page.getByRole("button", {name: "Import", exact: true}).click()
    const drawer = page.getByRole("dialog")
    await drawer.getByRole("textbox", {name: "Integrity Check (SHA-256)"}).fill(CHECKSUM)
    const upload = page.waitForRequest(
        (request) => request.url() === url && request.method() === "PUT"
    )
    await drawer.locator('input[type="file"]').setInputFiles({
        name: encrypted ? "event.ezip" : "event.json",
        mimeType: encrypted ? "application/ezip" : "application/json",
        buffer: CONTENT,
    })
    if (encrypted) {
        const password = page
            .getByRole("dialog")
            .filter({has: page.getByText("Decryption Password", {exact: true})})
        await password.locator('input[type="password"]').fill("synthetic archive password")
        await password.getByRole("button", {name: "Ok", exact: true}).click()
    }
    const request = await upload
    await whileUploading?.()
    expect(request.postDataBuffer()).toEqual(CONTENT)
    expect(request.headers()["content-type"]).toBe(
        encrypted ? "application/ezip" : "application/json"
    )
    expect(portal.graphql.callsTo("GetUploadUrl").map(({variables}) => variables)).toEqual([
        {
            name: encrypted ? "event.ezip" : "event.json",
            media_type: encrypted ? "application/ezip" : "application/json",
            size: 16,
            is_public: false,
        },
    ])
    await expect.poll(() => portal.graphql.callsTo("ImportElectionEvent").length).toBe(1)
    expect(portal.graphql.callsTo("ImportElectionEvent")[0].variables).toEqual({
        tenantId: TENANT_ID,
        documentId: DOCUMENT_ID,
        checkOnly: true,
        password: encrypted ? "synthetic archive password" : "",
    })
    return drawer
}

test("creates an event then refreshes the scoped tree after task completion", async ({
    page,
    portal,
}) => {
    const workflow = eventWorkflow(portal, "create")
    await boot(page, portal)
    await page.getByRole("button", {name: "Add", exact: true}).click()
    const drawer = page.getByRole("dialog")
    await drawer.getByRole("textbox", {name: "Name", exact: true}).fill("Created council")
    await drawer
        .getByRole("textbox", {name: "Description", exact: true})
        .fill("Annual council election")
    await drawer.getByRole("button", {name: "Save", exact: true}).click()
    await expect.poll(() => portal.graphql.callsTo("CreateElectionEvent").length).toBe(1)
    expect(portal.graphql.callsTo("CreateElectionEvent")[0].variables).toEqual({
        electionEvent: {
            id: expect.stringMatching(
                /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/
            ),
            tenant_id: TENANT_ID,
            name: "Created council",
            description: "Annual council election",
            encryption_protocol: "RSA256",
            is_archived: false,
            presentation: {
                language_conf: {enabled_language_codes: ["en"], default_language_code: "en"},
                i18n: {
                    en: {name: "Created council", description: "Annual council election"},
                    es: {name: "Created council", description: "Annual council election"},
                    cat: {name: "Created council", description: "Annual council election"},
                    fr: {name: "Created council", description: "Annual council election"},
                    tl: {name: "Created council", description: "Annual council election"},
                    gl: {name: "Created council", description: "Annual council election"},
                    nl: {name: "Created council", description: "Annual council election"},
                    eu: {name: "Created council", description: "Annual council election"},
                },
            },
        },
    })
    await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
    workflow.complete("SUCCESS")
    await page.clock.runFor(250)
    await expect(page).toHaveURL(
        new RegExp(`/sequent_backend_election_event/${workflow.eventId()}`)
    )
    await expect(page.getByText("Created council", {exact: true}).first()).toBeVisible()
    expect(portal.graphql.callsTo("election_events_tree").at(-1)?.variables).toEqual({
        tenantId: TENANT_ID,
        isArchived: false,
    })
})
for (const encrypted of [false, true])
    test(`imports ${encrypted ? "encrypted" : "plain"} archives through validation, checksum and successful task polling`, async ({
        page,
        portal,
    }) => {
        const workflow = eventWorkflow(portal, "import")
        let releaseUpload!: () => void
        const heldUpload = new Promise<void>((resolve) => {
            releaseUpload = resolve
        })
        if (encrypted) {
            await page.route(workflow.url, async (route) => {
                await heldUpload
                await route.fallback()
            })
        }
        const drawer = await selectArchive(
            page,
            portal,
            workflow.url,
            encrypted,
            encrypted
                ? async () => {
                      try {
                          const uploading = page.getByRole("dialog").filter({
                              has: page.getByRole("textbox", {
                                  name: "Integrity Check (SHA-256)",
                              }),
                          })
                          await expect(uploading.getByRole("progressbar")).toBeVisible()
                          await expect(
                              uploading.getByRole("textbox", {name: "Integrity Check (SHA-256)"})
                          ).toBeDisabled()
                          await expect(uploading.locator('input[type="file"]')).toBeDisabled()
                          await expect(
                              uploading.getByRole("button", {name: "Cancel", exact: true})
                          ).toBeDisabled()
                          const transfer = await page.evaluateHandle(() => {
                              const data = new DataTransfer()
                              data.items.add(
                                  new File(["replacement"], "replacement.json", {
                                      type: "application/json",
                                  })
                              )
                              return data
                          })
                          try {
                              await uploading
                                  .locator(".drop-file-dropzone")
                                  .dispatchEvent("drop", {dataTransfer: transfer})
                          } finally {
                              await transfer.dispose()
                          }
                          expect(portal.graphql.callsTo("GetUploadUrl")).toHaveLength(1)
                          expect(portal.graphql.callsTo("ImportElectionEvent")).toEqual([])
                      } finally {
                          releaseUpload()
                      }
                  }
                : undefined
        )
        await expect(drawer.getByRole("button", {name: "Import", exact: true})).toBeEnabled()
        await drawer.getByRole("button", {name: "Import", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("ImportElectionEvent").length).toBe(2)
        expect(portal.graphql.callsTo("ImportElectionEvent")[1].variables).toEqual({
            tenantId: TENANT_ID,
            documentId: DOCUMENT_ID,
            password: encrypted ? "synthetic archive password" : "",
            sha256: CHECKSUM,
        })
        await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
        workflow.complete("SUCCESS")
        await page.clock.runFor(250)
        await expect(page).toHaveURL(
            new RegExp(`/sequent_backend_election_event/${workflow.eventId()}`)
        )
        await expect(page.getByText("Imported council", {exact: true}).first()).toBeVisible()
    })
test("imports a plain archive after cancelling an encrypted archive without reusing its password or MIME type", async ({
    page,
    portal,
}) => {
    const workflow = eventWorkflow(portal, "import")
    await boot(page, portal)
    await page.getByRole("button", {name: "Import", exact: true}).click()
    const drawer = page.getByRole("dialog").filter({
        has: page.getByRole("textbox", {name: "Integrity Check (SHA-256)"}),
    })
    await drawer.getByRole("textbox", {name: "Integrity Check (SHA-256)"}).fill(CHECKSUM)
    await drawer.locator('input[type="file"]').setInputFiles({
        name: "cancelled.ezip",
        mimeType: "application/ezip",
        buffer: CONTENT,
    })
    const passwordDialog = page.getByRole("dialog", {name: "Decryption Password", exact: true})
    await passwordDialog.locator('input[type="password"]').fill("cancelled archive password")
    await passwordDialog.press("Escape")
    await expect(passwordDialog).toBeHidden()
    expect(portal.graphql.callsTo("GetUploadUrl")).toEqual([])
    expect(portal.graphql.callsTo("ImportElectionEvent")).toEqual([])

    const upload = page.waitForRequest(
        (request) => request.url() === workflow.url && request.method() === "PUT"
    )
    await drawer.locator('input[type="file"]').setInputFiles({
        name: "event.json",
        mimeType: "application/json",
        buffer: CONTENT,
    })
    const request = await upload
    expect(request.postDataBuffer()).toEqual(CONTENT)
    expect(request.headers()["content-type"]).toBe("application/json")
    await expect.poll(() => portal.graphql.callsTo("ImportElectionEvent").length).toBe(1)
    expect({
        upload: portal.graphql.callsTo("GetUploadUrl").map(({variables}) => variables),
        validation: portal.graphql.callsTo("ImportElectionEvent").map(({variables}) => variables),
    }).toEqual({
        upload: [{name: "event.json", media_type: "application/json", size: 16, is_public: false}],
        validation: [{tenantId: TENANT_ID, documentId: DOCUMENT_ID, checkOnly: true, password: ""}],
    })
    await expect(drawer.getByRole("button", {name: "Import", exact: true})).toBeEnabled()
    await drawer.getByRole("button", {name: "Import", exact: true}).click()
    await expect.poll(() => portal.graphql.callsTo("ImportElectionEvent").length).toBe(2)
    expect(portal.graphql.callsTo("ImportElectionEvent")[1].variables).toEqual({
        tenantId: TENANT_ID,
        documentId: DOCUMENT_ID,
        password: "",
        sha256: CHECKSUM,
    })
    await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
    workflow.complete("SUCCESS")
    await page.clock.runFor(250)
    await expect(page).toHaveURL(
        new RegExp(`/sequent_backend_election_event/${workflow.eventId()}`)
    )
    await expect(page.getByText("Imported council", {exact: true}).first()).toBeVisible()
})
test("shows failed import task logs without navigating to an event", async ({page, portal}) => {
    const workflow = eventWorkflow(portal, "import")
    const drawer = await selectArchive(page, portal, workflow.url, false)
    await drawer.getByRole("button", {name: "Import", exact: true}).click()
    await expect.poll(() => portal.graphql.callsTo("GetTaskById").length).toBeGreaterThan(0)
    workflow.complete("FAILED")
    await page.clock.runFor(250)
    await expect(page.getByText("Synthetic event task failed", {exact: true})).toBeVisible()
    await expect(page).not.toHaveURL(/sequent_backend_election_event\//)
})
test("rejects invalid archive validation before an import task starts", async ({page, portal}) => {
    const workflow = eventWorkflow(portal, "import", true)
    const drawer = await selectArchive(page, portal, workflow.url, false)
    await expect(
        drawer.getByText("Synthetic archive validation failed", {exact: true})
    ).toBeVisible()
    await expect(drawer.getByRole("button", {name: "Import", exact: true})).toBeDisabled()
    expect(portal.graphql.callsTo("ImportElectionEvent")).toHaveLength(1)
    expect(portal.graphql.callsTo("GetTaskById")).toHaveLength(0)
})

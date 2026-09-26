// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {FIXED_TIME, IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect} from "../fixtures"
import {VOTER_TAB_ROLES, expectRole, mockVoters, openVoters, taskRow} from "./data"

const SYNC_ROLES = [...VOTER_TAB_ROLES, "election-event-voter-list-reconciliation"]
const FILE = Buffer.from("VoterID,CountyMun,Ward,Channel,Deleted\nV-100,01,3,NONE,false\n")
const GENERATED_AT = Date.parse(FIXED_TIME) / 1000

interface Round {
    upload: string
    task: string
    diff: string
    applyTask?: string
    status?: "SUCCESS" | "FAILED"
    applyAnnotations?: Record<string, unknown>
    envelope: {
        sequence: number
        apply_allowed: boolean
        external_patch_document_id: string | null
        items: Record<string, unknown>[]
    }
}

const round = (index: number, envelope: Round["envelope"], extra: Partial<Round> = {}): Round => ({
    upload: `aaaaaaaa-0000-4000-8000-00000000000${index}`,
    task: `bbbbbbbb-0000-4000-8000-00000000000${index}`,
    diff: `cccccccc-0000-4000-8000-00000000000${index}`,
    envelope,
    ...extra,
})

/** Serves one upload, diff task and diff document per round, in order. */
function mockSync(portal: PortalServices, rounds: Round[]) {
    mockVoters(portal)
    let current = -1
    const byId = (id: unknown) => rounds.find((r) => r.task === id || r.applyTask === id)
    portal.graphql.on("GetUploadUrl", () => {
        current += 1
        const {upload} = rounds[current]
        return {
            data: {
                get_upload_url: {
                    url: portal.s3.presign(`documents/${upload}`, `sync-${current}`),
                    document_id: upload,
                },
            },
        }
    })
    portal.s3.override(
        (request) => request.method === "PUT" && request.key.startsWith("documents/aaaaaaaa"),
        {status: 200}
    )
    portal.graphql.on("CreateExternalReconciliationImport", () => ({
        data: {create_external_reconciliation_import: {task_execution: {id: rounds[current].task}}},
    }))
    portal.graphql.on("ApplyExternalReconciliationChanges", () => ({
        data: {
            apply_external_reconciliation_changes: {
                task_execution: {id: rounds[current].applyTask},
            },
        },
    }))
    portal.graphql.on("GetTaskById", ({variables}) => {
        const owner = byId(variables.task_id)!
        const apply = owner.applyTask === variables.task_id
        return {
            data: {
                sequent_backend_tasks_execution: [
                    taskRow(
                        apply ? "APPLY_RECONCILIATION_PATCH" : "GENERATE_RECONCILIATION_PATCHES",
                        apply ? "SUCCESS" : (owner.status ?? "SUCCESS"),
                        apply ? (owner.applyAnnotations ?? {}) : {document_id: owner.diff},
                        String(variables.task_id)
                    ),
                ],
            },
        }
    })
    portal.graphql.on("FetchDocument", ({variables}) => {
        const owner = rounds.find((r) => r.diff === variables.documentId)
        const key = `documents/${String(variables.documentId)}`
        if (owner)
            portal.s3.putJson("private", key, {...owner.envelope, generated_at: GENERATED_AT})
        return {data: {fetchDocument: {url: portal.s3.presign(key, "sync-document")}}}
    })
    portal.graphql.on("GetDocument", () => ({
        data: {sequent_backend_document: [{name: "patch.csv", annotations: {}}]},
    }))
}

async function dropFile(page: Page, name = "reconciliation.csv") {
    await page.getByRole("button", {name: "Ext. voters sync", exact: true}).click()
    const drawer = page.getByRole("dialog")
    await expect(drawer.getByText("External reconciliation sync", {exact: true})).toBeVisible()
    await drawer
        .locator('input[type="file"]')
        .setInputFiles({name, mimeType: "text/csv", buffer: FILE})
    return drawer
}

const item = (
    voter: string,
    target: "datafix" | "sequent",
    category: string,
    field: Record<string, unknown> | null,
    failure_reason: string | null = null
) => ({voter_username: voter, target, category, field, failure_reason})

test.describe("voter list reconciliation operator", () => {
    // Wide enough for the diff tables to render their Reason column.
    test.use({roles: SYNC_ROLES, viewport: {width: 1920, height: 1080}})

    test("holds apply back while the external system still has a patch to take", async ({
        page,
        portal,
    }) => {
        const patchId = "dddddddd-0000-4000-8000-000000000001"
        const first = round(1, {
            sequence: 7,
            apply_allowed: true,
            external_patch_document_id: patchId,
            items: [
                item("V-100", "datafix", "VOTED_INTERNET", {Channel: ["NONE", "INTERNET"]}),
                item("V-200", "sequent", "PROFILE_UPDATE", {Ward: ["3", "4"]}),
                item("V-300", "sequent", "ROW_FAILURE", null, "CountyMun mismatch"),
            ],
        })
        mockSync(portal, [first])
        await openVoters(page, portal)
        const upload = page.waitForRequest((request) => request.method() === "PUT")
        const drawer = await dropFile(page)
        const put = await upload
        expect(put.postDataBuffer()).toEqual(FILE)
        expect(put.headers()["content-type"]).toBe("text/csv")
        await expect(
            drawer.getByText("reconciliation.csv - Sequence 7, generated 1/15/2026, 12:00:00 PM", {
                exact: true,
            })
        ).toBeVisible()
        await expect(
            drawer.getByText(
                "1 row(s) could not be reconciled safely and are excluded from both diffs - see below for details."
            )
        ).toBeVisible()
        await expect(
            drawer.getByRole("row", {name: /V-300 Row Row failure NONE NONE CountyMun mismatch/})
        ).toBeVisible()
        await expect(drawer.getByText("External diff (1)", {exact: true})).toBeVisible()
        await expect(
            drawer.getByRole("row", {name: /V-100 Channel Voted via Internet NONE INTERNET/})
        ).toBeVisible()
        await expect(drawer.getByText("Sequent diff (1)", {exact: true})).toBeVisible()
        await expect(drawer.getByRole("row", {name: /V-200 Ward Profile update 3 4/})).toBeVisible()
        await expect(drawer.getByRole("button", {name: "Apply", exact: true})).toBeDisabled()
        const downloading = page.waitForEvent("download")
        await drawer.getByRole("button", {name: "Download external patch", exact: true}).click()
        expect((await downloading).suggestedFilename()).toBe(
            `external-reconciliation-${patchId}.csv`
        )
        await drawer.getByRole("button", {name: "Back", exact: true}).click()
        await expect(drawer.locator('input[type="file"]')).toBeAttached()
        expect(portal.graphql.callsTo("GetUploadUrl")[0].variables).toEqual({
            name: "reconciliation.csv",
            media_type: "text/csv",
            size: FILE.length,
            is_public: false,
            election_event_id: IDS.event,
        })
        expect(
            portal.graphql.callsTo("CreateExternalReconciliationImport").map((c) => c.variables)
        ).toEqual([{election_event_id: IDS.event, document_id: first.upload}])
        expect(portal.graphql.callsTo("FetchDocument").map((c) => c.variables)).toEqual([
            {electionEventId: IDS.event, documentId: first.diff},
            {electionEventId: IDS.event, documentId: patchId},
        ])
        expect(portal.graphql.callsTo("ApplyExternalReconciliationChanges")).toHaveLength(0)
        expectRole(portal, "CreateExternalReconciliationImport", "admin-user")
    })

    test("applies the Sequent-side diff after confirming what it changes", async ({
        page,
        portal,
    }) => {
        const clean = round(
            1,
            {
                sequence: 8,
                apply_allowed: true,
                external_patch_document_id: null,
                items: [
                    item("V-100", "sequent", "DISABLED_DELETE_CALL", {Enabled: [true, false]}),
                    item("V-101", "sequent", "DISABLED_DELETE_CALL", {Enabled: [true, false]}),
                    item("V-102", "sequent", "VOTED_OTHER_CHANNEL", {
                        KeycloakUA: [
                            {"voted-channel": "NONE"},
                            {"voted-channel": "PAPER", "disable-comment": "Voted on paper"},
                        ],
                    }),
                    item("V-103", "sequent", "VOTER_ADDED", {DoB: [null, "1990-05-01"]}),
                ],
            },
            {
                applyTask: "eeeeeeee-0000-4000-8000-000000000001",
                applyAnnotations: {
                    reconciliation_row_failures: [
                        {voter_id: "V-101", reason: "Voter already voted online"},
                    ],
                    reconciliation_row_failure_count: 3,
                    reconciliation_row_failures_truncated: true,
                },
            }
        )
        mockSync(portal, [clean])
        await openVoters(page, portal)
        const drawer = await dropFile(page)
        await expect(drawer.getByText("External diff", {exact: true})).toBeVisible()
        await expect(drawer.getByText("No external-side differences.", {exact: true})).toBeVisible()
        await expect(
            drawer.getByRole("row", {
                name: /V-102 Disable Comment Voted via other channel NONE Voted on paper/,
            })
        ).toBeVisible()
        await drawer.getByRole("button", {name: "Apply", exact: true}).click()
        const confirm = page.getByRole("dialog", {name: "Confirm reconciliation changes"})
        await expect(
            confirm.getByText(
                "This will apply changes that marks 1 voter(s) as voted via other channels, disables 2 voter(s), adds 1 voter(s).",
                {exact: true}
            )
        ).toBeVisible()
        expect(portal.graphql.callsTo("ApplyExternalReconciliationChanges")).toHaveLength(0)
        await confirm.getByRole("button", {name: "Apply changes", exact: true}).click()
        await expect(
            drawer.getByText(
                "3 row(s) were excluded from this round and need manual follow-up - see below for details.",
                {exact: true}
            )
        ).toBeVisible()
        await expect(
            drawer.getByText(
                "Showing the first 1 of 3 row failures. Resolve the shared cause and retry to see any remaining failures.",
                {exact: true}
            )
        ).toBeVisible()
        await expect(
            drawer.getByRole("row", {name: /V-101.*Voter already voted online/})
        ).toBeVisible()
        expect(
            portal.graphql.callsTo("ApplyExternalReconciliationChanges").map((c) => c.variables)
        ).toEqual([{election_event_id: IDS.event, diff_document_id: clean.diff}])
        await drawer.getByRole("button", {name: "Close", exact: true}).click()
        await expect(drawer).toHaveCount(0)
        expectRole(portal, "ApplyExternalReconciliationChanges", "admin-user")
    })

    test("returns to the file drop when the diff calculation fails", async ({page, portal}) => {
        mockSync(portal, [
            round(
                1,
                {sequence: 9, apply_allowed: true, external_patch_document_id: null, items: []},
                {status: "FAILED"}
            ),
        ])
        await openVoters(page, portal)
        const drawer = await dropFile(page)
        await expect(
            drawer.getByText(
                "Failed to calculate the reconciliation diff - see the task widget for details.",
                {exact: true}
            )
        ).toBeVisible()
        await expect(drawer.locator('input[type="file"]')).toBeAttached()
        expect(portal.graphql.callsTo("FetchDocument")).toHaveLength(0)
    })

    test("treats an already applied sequence as a check that is never applied", async ({
        page,
        portal,
    }) => {
        mockSync(portal, [
            round(1, {
                sequence: 7,
                apply_allowed: false,
                external_patch_document_id: null,
                items: [item("V-200", "sequent", "PROFILE_UPDATE", {Ward: ["3", "4"]})],
            }),
            round(2, {
                sequence: 7,
                apply_allowed: false,
                external_patch_document_id: null,
                items: [],
            }),
        ])
        await openVoters(page, portal)
        const drawer = await dropFile(page)
        await expect(
            drawer.getByText(
                "This is a convergence check for an already applied Sequence. Differences are shown for follow-up, but this round cannot be applied again.",
                {exact: true}
            )
        ).toBeVisible()
        await expect(drawer.getByRole("button", {name: "Apply", exact: true})).toBeDisabled()
        await drawer.getByRole("button", {name: "Back", exact: true}).click()
        await drawer
            .locator('input[type="file"]')
            .setInputFiles({name: "again.csv", mimeType: "text/csv", buffer: FILE})
        await expect(
            drawer.getByText("No differences - the two systems are already in sync.", {
                exact: true,
            })
        ).toBeVisible()
        await drawer.getByRole("button", {name: "Next", exact: true}).click()
        await expect(drawer.getByRole("button", {name: "Start over", exact: true})).toBeVisible()
        expect(portal.graphql.callsTo("ApplyExternalReconciliationChanges")).toHaveLength(0)
        expect(portal.graphql.callsTo("CreateExternalReconciliationImport")).toHaveLength(2)
    })
})

test.describe("voter operator without the reconciliation permission", () => {
    test.use({roles: VOTER_TAB_ROLES})

    test("does not see the external voter sync", async ({page, portal}) => {
        mockVoters(portal)
        await openVoters(page, portal)
        await expect(page.getByRole("cell", {name: "alice", exact: true})).toBeVisible()
        await expect(page.getByRole("button", {name: "Ext. voters sync"})).toHaveCount(0)
    })
})

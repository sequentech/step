// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {PortalServices} from "@sequentech/ui-test-kit/adapters/playwright"
import {IDS} from "@sequentech/ui-test-kit/fixtures"
import {test, expect, TENANT_ID} from "../fixtures"
import {BASE_ROLES, CONTENT_IDS, eventPage, expectRole, table} from "./data"

const SECOND_DOCUMENT_ID = "a0000000-0000-4000-8000-000000000002"
test.use({roles: [...BASE_ROLES, "document-read", "document-write"]})

function documents(portal: PortalServices) {
    eventPage(portal)
    const rows = table(portal, "sequent_backend_document", [
        {
            id: CONTENT_IDS.document,
            tenant_id: TENANT_ID,
            election_event_id: IDS.event,
            name: "guide.txt",
            media_type: "text/plain",
            size: 16,
            is_public: false,
            labels: {},
            annotations: {},
        },
        {
            id: SECOND_DOCUMENT_ID,
            tenant_id: TENANT_ID,
            election_event_id: IDS.event,
            name: "expired.txt",
            media_type: "text/plain",
            size: 16,
            is_public: false,
            labels: {},
            annotations: {},
        },
    ])
    const key = `${TENANT_ID}/${IDS.event}/guide.txt`
    portal.s3.putBytes("private", key, Buffer.from("Voting guide text"), "text/plain")
    const url = portal.s3.presign(key, "document-download")
    portal.graphql.on("FetchDocument", ({variables}) =>
        variables.documentId === CONTENT_IDS.document
            ? {data: {fetchDocument: {url}}}
            : {errors: [{message: "document expired"}]}
    )
    return {rows, key, url}
}

test("opens a listed document and downloads its source", async ({page, portal}) => {
    const {url} = documents(portal)
    await page.goto(`${portal.origin}/sequent_backend_document?lang=en`)
    await expect(page.getByRole("heading", {name: "Documents", exact: true})).toBeVisible()
    await page.getByRole("cell", {name: "guide.txt", exact: true}).click()
    await expect(page).toHaveURL(
        new RegExp(`/sequent_backend_document/${CONTENT_IDS.document}/show`)
    )
    await expect(page.getByText("Size (bytes)", {exact: true})).toBeVisible()
    const download = page.waitForEvent("download")
    await page.getByRole("button", {name: "Download Document", exact: true}).click()
    const file = await download
    expect(file.suggestedFilename()).toBe("guide.txt")
    expect(file.url()).toBe(url)
    expect(portal.graphql.callsTo("FetchDocument").map(({variables}) => variables)).toEqual([
        {electionEventId: IDS.event, documentId: CONTENT_IDS.document},
    ])
    expectRole(portal, "sequent_backend_document", "document-read")
})

test("reports a rejected document download after a successful download", async ({page, portal}) => {
    documents(portal)
    await page.goto(`${portal.origin}/sequent_backend_document?lang=en`)
    await page.getByRole("cell", {name: "guide.txt", exact: true}).click()
    const download = page.waitForEvent("download")
    await page.getByRole("button", {name: "Download Document", exact: true}).click()
    await download
    await page.getByRole("cell", {name: "expired.txt", exact: true}).click()
    await page.getByRole("button", {name: "Download Document", exact: true}).click()
    await expect.poll(() => portal.graphql.callsTo("FetchDocument").length).toBe(2)
    expect(portal.graphql.callsTo("FetchDocument").map(({variables}) => variables)).toEqual([
        {electionEventId: IDS.event, documentId: CONTENT_IDS.document},
        {electionEventId: IDS.event, documentId: SECOND_DOCUMENT_ID},
    ])
    test.fail(
        true,
        "ShowDocument keeps the download spinner after FetchDocument rejects without showing the error"
    )
    await expect(page.getByRole("alert")).toContainText("document expired", {timeout: 2000})
})

test("opens a document directly without a cached record", async ({page, portal}) => {
    documents(portal)
    const errors: string[] = []
    page.on("console", (message) => {
        if (message.type() === "error") errors.push(message.text())
    })
    await page.goto(`${portal.origin}/sequent_backend_document?lang=en`)
    await page.getByRole("cell", {name: "guide.txt", exact: true}).click()
    const downloadButton = page.getByRole("button", {name: "Download Document", exact: true})
    await expect(downloadButton).toBeVisible()
    // Reload removes React Admin's cached list record before rendering the same detail URL.
    await page.reload()
    const errorAlert = page.getByRole("alert").filter({hasText: "Something went wrong"})
    await expect(downloadButton.or(errorAlert)).toBeVisible()
    expect(
        errors.filter(
            (message) =>
                !message.startsWith(
                    "TypeError: Cannot read properties of undefined (reading 'labels')"
                )
        )
    ).toEqual([])
    test.fail(
        true,
        "ShowDocument renders JsonField before its record exists, so a direct link crashes while reading labels"
    )
    await expect(page.getByRole("button", {name: "Download Document", exact: true})).toBeVisible({
        timeout: 2000,
    })
    await expect(page.getByRole("alert").filter({hasText: "Something went wrong"})).toHaveCount(0)
})

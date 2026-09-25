// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import jsQR from "jsqr"
import {readFile, mkdir, writeFile, rm} from "node:fs/promises"
import {resolve, dirname} from "node:path"
import {test, expect, eventPath, IDS, FIXED_TIME, electionFixture} from "./fixtures"
import {review} from "./flow"

const documentId = "a0000000-0000-4000-8000-000000000001"
const pdf = "%PDF-1.4\nSynthetic receipt\n%%EOF\n"

test("receipt QR independently decodes to the scoped locator and print waits for its document", async ({
    page,
    portal,
}) => {
    portal.settings.QUERY_POLL_INTERVAL_MS = 1000
    portal.graphql.on("createBallotReceipt", ({variables}) => ({
        data: {
            create_ballot_receipt: {
                id: documentId,
                ballot_id: variables.ballot_id,
                status: "pending",
            },
        },
    }))
    let documentReady = false
    portal.graphql.on("GetDocument", () => ({
        data: {
            sequent_backend_document: documentReady
                ? [
                      {
                          id: documentId,
                          tenant_id: IDS.tenant,
                          election_event_id: IDS.event,
                          name: "receipt.pdf",
                          media_type: "application/pdf",
                          size: pdf.length,
                          is_public: true,
                          created_at: FIXED_TIME,
                          last_updated_at: FIXED_TIME,
                          labels: {},
                          annotations: {},
                      },
                  ]
                : [],
        },
    }))
    const key = `tenant-${IDS.tenant}/document-${documentId}/ballot_receipt_${IDS.event}.pdf`
    const receiptUrl = portal.s3.putBytes(
        "public",
        key,
        new TextEncoder().encode(pdf),
        "application/pdf"
    )
    // Chromium may fetch an <a download> in its browser process, outside page routing.
    // Keep the same disposable fixture available on the owned loopback server.
    const receiptFile = resolve(__dirname, "../../dist", `.${new URL(receiptUrl).pathname}`)
    await mkdir(dirname(receiptFile), {recursive: true})
    await writeFile(receiptFile, pdf)
    try {
        await review(page, portal)
        await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
        await expect(page).toHaveURL(/\/confirmation/)
        const ballotId = await page.getByTestId("ballot-id").first().innerText()
        const pixels = await page.locator("svg.qr-code-svg").evaluate(async (svg) => {
            const image = new Image()
            image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(new XMLSerializer().serializeToString(svg))}`
            await image.decode()
            const canvas = document.createElement("canvas")
            canvas.width = canvas.height = 544
            const context = canvas.getContext("2d")!
            context.fillStyle = "white"
            context.fillRect(0, 0, 544, 544)
            context.imageSmoothingEnabled = false
            context.drawImage(image, 16, 16, 512, 512)
            return Array.from(context.getImageData(0, 0, 544, 544).data)
        })
        const locatorUrl = `${portal.origin}${eventPath}/election/${IDS.election}/ballot-locator/${ballotId}`
        expect(jsQR(new Uint8ClampedArray(pixels), 544, 544)?.data).toBe(locatorUrl)
        await page.getByRole("button", {name: "Print", exact: true}).click()
        await expect.poll(() => portal.graphql.callsTo("GetDocument").length).toBeGreaterThan(0)
        await expect(page.getByRole("button", {name: "Print", exact: true})).toBeDisabled()
        documentReady = true
        const pendingDownload = page.waitForEvent("download")
        await page.clock.runFor(1100)
        const download = await pendingDownload
        expect(await readFile((await download.path())!, "utf8")).toBe(pdf)
        expect(portal.graphql.callsTo("createBallotReceipt")).toHaveLength(1)
        expect(portal.graphql.callsTo("createBallotReceipt")[0].variables).toEqual({
            ballot_id: ballotId,
            ballot_tracker_url: locatorUrl,
            election_event_id: IDS.event,
            tenant_id: IDS.tenant,
            election_id: IDS.election,
        })
        await expect(page.getByRole("button", {name: "Print", exact: true})).toBeEnabled()
    } finally {
        await rm(dirname(receiptFile), {recursive: true, force: true})
    }
})

test("demo receipt explains that no receipt exists and sends no creation mutation", async ({
    page,
    portal,
}) => {
    portal.data = electionFixture({demo: true})
    portal.publish()
    await review(page, portal, true)
    await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
    await expect(page).toHaveURL(/\/confirmation/)
    await page.getByRole("button", {name: "Print", exact: true}).click()
    await expect(page.getByRole("dialog")).toContainText(/demo/i)
    expect(portal.graphql.callsTo("createBallotReceipt")).toEqual([])
})

test("receipt generation failure offers a retry instead of polling indefinitely", async ({
    page,
    portal,
}) => {
    portal.settings.QUERY_POLL_INTERVAL_MS = 1000
    portal.settings.POLLING_DURATION_TIMEOUT = 5000
    portal.graphql.on("createBallotReceipt", ({variables}) => ({
        data: {
            create_ballot_receipt: {
                id: documentId,
                ballot_id: variables.ballot_id,
                status: "pending",
            },
        },
    }))
    portal.graphql.on("GetDocument", () => ({
        errors: [{message: "Synthetic receipt generation failed"}],
    }))
    await review(page, portal)
    await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
    await expect(page).toHaveURL(/\/confirmation/)
    await page.getByRole("button", {name: "Print", exact: true}).click()
    await expect.poll(() => portal.graphql.callsTo("GetDocument").length).toBeGreaterThan(0)
    for (let attempt = 0; attempt < 6; attempt++) {
        const before = portal.graphql.callsTo("GetDocument").length
        await page.clock.runFor(1100)
        await expect
            .poll(() => portal.graphql.callsTo("GetDocument").length)
            .toBeGreaterThan(before)
    }
    expect(portal.graphql.callsTo("createBallotReceipt")).toHaveLength(1)
    expect(portal.graphql.callsTo("GetDocument").length).toBeGreaterThan(5)
    await expect(page.getByRole("button", {name: "Print", exact: true})).toBeDisabled()
    test.fail(
        true,
        "ConfirmationScreen never sets its receipt error dialog and does not bound GetDocument polling after failure"
    )
    await expect(page.getByRole("dialog")).toBeVisible({timeout: 1000})
})

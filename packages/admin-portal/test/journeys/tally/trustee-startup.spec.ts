// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {expect} from "@playwright/test"
import {test} from "../fixtures"
import {readFile} from "node:fs/promises"

test("provides native trustee setup and custody commands", async ({page, portal}) => {
    await page.goto(`${portal.origin}/trustee?lang=en`)
    await expect(page.getByRole("heading", {name: "Trustee dashboard"})).toBeVisible()
    await expect(
        page.locator("pre code").filter({hasText: "trustee generate --trustee-config"})
    ).toBeVisible()
    await expect(
        page.locator("pre code").filter({hasText: "trustee attest --request"})
    ).toBeVisible()
})

for (const width of [1280, 390])
    test(`generates and restores browser trustee keys at ${width}px`, async ({
        page,
        portal,
    }, testInfo) => {
        await page.setViewportSize({width, height: 900})
        portal.trustee = "browser-trustee"
        await page.goto(`${portal.origin}/trustee?lang=en`)
        await expect(page.getByRole("heading", {name: "Trustee dashboard"})).toBeVisible()
        await page.screenshot({path: testInfo.outputPath("dashboard-initial.png"), fullPage: true})
        await page.getByRole("button", {name: "Generate browser keys"}).click()
        await expect(page.getByLabel("Signing public key", {exact: true})).not.toHaveValue("")
        await expect(page.getByLabel("Share encryption public key", {exact: true})).not.toHaveValue(
            ""
        )
        const download = page.waitForEvent("download")
        await page.getByRole("button", {name: "Download private keys"}).click()
        const keys = await download
        expect(keys.suggestedFilename()).toBe("browser-trustee-private-keys.toml")
        const path = await keys.path()
        if (!path) throw new Error("No local keys download")
        const keyFile = await readFile(path, "utf8")
        expect(keyFile).toMatch(/signing_key = "[^"\n]+"/)
        expect(keyFile).toMatch(/share_encryption_key = "[^"\n]+"/)
        const signing = await page.getByLabel("Signing public key", {exact: true}).inputValue()
        await page.getByRole("button", {name: "Start browser trustee"}).scrollIntoViewIfNeeded()
        await page.screenshot({path: testInfo.outputPath("dashboard-ready.png"), fullPage: true})
        await page.locator("pre code").last().scrollIntoViewIfNeeded()
        await page.screenshot({
            path: testInfo.outputPath("dashboard-local-tool.png"),
            fullPage: true,
        })
        const backupDownload = page.waitForEvent("download")
        await page.getByRole("button", {name: "Back up browser sessions"}).click()
        const backup = await backupDownload
        expect(backup.suggestedFilename()).toBe("browser-trustee-browser-sessions.json")
        const backupPath = await backup.path()
        if (!backupPath) throw new Error("No browser session backup")
        expect(JSON.parse(await readFile(backupPath, "utf8"))).toMatchObject({
            version: 1,
            keys: keyFile,
            stores: [],
        })
        await page.getByRole("button", {name: "Lock keys"}).click()
        await expect(page.getByLabel("Signing public key", {exact: true})).toHaveCount(0)
        await page.getByLabel("Restore browser backup").setInputFiles(backupPath)
        await expect(page.getByLabel("Signing public key", {exact: true})).toHaveValue(signing)
        // The request is public. The real WASM checks both keys before signing.
        const request = {
            tenant_id: "tenant",
            election_event_id: "event",
            keys_ceremony_id: "ceremony",
            tally_session_id: null,
            trustee_name: "browser-trustee",
            signing_public_key: signing,
            share_encryption_public_key: await page
                .getByLabel("Share encryption public key", {exact: true})
                .inputValue(),
        }
        const proofDownload = page.waitForEvent("download")
        await page.getByLabel("Custody request").setInputFiles({
            name: "request.json",
            mimeType: "application/json",
            buffer: Buffer.from(JSON.stringify(request)),
        })
        const proofFile = await proofDownload
        expect(proofFile.suggestedFilename()).toBe("browser-trustee-custody-proof.json")
        const proofPath = await proofFile.path()
        if (!proofPath) throw new Error("No custody proof download")
        const proof = JSON.parse(await readFile(proofPath, "utf8"))
        expect(proof.request).toEqual(request)
        expect(Buffer.from(proof.signature, "base64")).toHaveLength(64)
        expect(Object.keys(proof).sort()).toEqual(["request", "signature"])
        await page.getByLabel("Custody request").setInputFiles({
            name: "request.json",
            mimeType: "application/json",
            buffer: Buffer.from(JSON.stringify({...request, trustee_name: "other-trustee"})),
        })
        await expect(page.getByRole("alert")).toContainText("another trustee")
        await page.getByLabel("Custody request").setInputFiles({
            name: "request.json",
            mimeType: "application/json",
            buffer: Buffer.from(
                JSON.stringify({...request, share_encryption_public_key: "foreign"})
            ),
        })
        await expect(page.getByRole("alert")).toContainText("do not match")
        expect(
            await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)
        ).toBe(true)

        await page.getByLabel("Import private keys").setInputFiles({
            name: "invalid.toml",
            mimeType: "text/plain",
            buffer: Buffer.from('signing_key = "broken"'),
        })
        await expect(page.getByRole("alert")).toContainText("share_encryption_key")
        await expect(page.getByLabel("Signing public key", {exact: true})).not.toHaveValue("")
    })

test("prevents the same trustee from running in two tabs", async ({page, context, portal}) => {
    portal.trustee = "browser-trustee"
    portal.graphql.on("ListTrusteeBoards", ({headers}) => {
        expect(headers.authorization).toMatch(/^Bearer /)
        expect(headers["x-hasura-role"]).toBe("trustee-ceremony")
        return {data: {list_trustee_boards: {boards: []}}}
    })
    await page.goto(`${portal.origin}/trustee?lang=en`)
    await expect(page.getByLabel("Platform URL")).toHaveValue(`${portal.origin}/v1/graphql`)
    await page.getByRole("button", {name: "Generate browser keys"}).click()
    await expect(page.getByLabel("Signing public key", {exact: true})).not.toHaveValue("")
    const download = page.waitForEvent("download")
    await page.getByRole("button", {name: "Download private keys"}).click()
    const keyPath = await (await download).path()
    if (!keyPath) throw new Error("No keys file")
    await page.getByRole("button", {name: "Start browser trustee"}).click()
    await expect(
        page.getByRole("status").filter({hasText: "Browser trustee running"})
    ).toBeVisible()
    const second = await context.newPage()
    await second.goto(`${portal.origin}/trustee?lang=en`)
    await second.getByLabel("Import private keys").setInputFiles(keyPath)
    await expect(second.getByLabel("Signing public key", {exact: true})).not.toHaveValue("")
    await second.getByRole("button", {name: "Start browser trustee"}).click()
    await expect(second.getByRole("alert")).toContainText("already running in another tab")
    await page.getByRole("button", {name: "Stop browser trustee"}).click()
    await expect(page.getByRole("button", {name: "Start browser trustee"})).toBeEnabled()
    await second.getByRole("button", {name: "Start browser trustee"}).click()
    await expect(
        second.getByRole("status").filter({hasText: "Browser trustee running"})
    ).toBeVisible()
    await second.getByRole("button", {name: "Stop browser trustee"}).click()
    await second.close()
})

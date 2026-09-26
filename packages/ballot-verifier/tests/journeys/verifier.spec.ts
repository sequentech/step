// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect, eventPath, realm} from "./fixtures"
import {signedBallot} from "./ballots"
import {scanPage} from "@sequentech/ui-test-kit/adapters/axe"

async function upload(page: Page, ballot: unknown, hash: string) {
    await page.getByTestId("drop-input-file").setInputFiles({
        name: "audited-ballot.json",
        mimeType: "application/json",
        buffer: Buffer.from(JSON.stringify(ballot)),
    })
    await page.getByRole("textbox", {name: "Ballot ID", exact: true}).fill(hash)
    await expect(page.getByRole("button", {name: "Next", exact: true})).toBeEnabled()
}
async function verify(page: Page, origin: string, ballot: unknown, hash: string) {
    await page.goto(`${origin}${eventPath}`)
    await upload(page, ballot, hash)
    await page.getByRole("button", {name: "Next", exact: true}).click()
    await expect(page.getByRole("heading", {name: "Verify your ballot selections"})).toBeVisible()
    await expect(page.getByText("Alice Example", {exact: true})).toBeVisible()
    await expect(page.getByText("Bob Example", {exact: true})).toHaveCount(0)
}

test("authentication uses the verifier realm, PKCE and a bearer-authenticated schema query", async ({
    page,
    portal,
}) => {
    await page.goto(`${portal.origin}${eventPath}`)
    await expect(page.getByRole("heading", {name: "Step 1: Import your ballot"})).toBeVisible()
    await expect(page.getByRole("button", {name: "Next", exact: true})).toBeDisabled()
    expect(portal.oidc.authorizations[0]).toMatchObject({
        realm,
        params: {client_id: "ballot-verifier", code_challenge_method: "S256", ui_locales: "en"},
    })
    expect(portal.oidc.tokenRequests[0]).toMatchObject({
        grantType: "authorization_code",
        pkce: "valid",
        status: 200,
    })
    expect(portal.graphql.callsTo("GetBallotStyles")).toHaveLength(1)
    expect(portal.graphql.callsTo("GetBallotStyles")[0].headers.authorization).toBe(
        `Bearer ${portal.oidc.lastIssued()?.access_token}`
    )
})

for (const multiple of [false, true]) {
    test(`signed ${multiple ? "multiple-contest" : "single-contest"} ballot reveals exactly the selected candidates`, async ({
        page,
        portal,
    }) => {
        const {ballot, hash} = await signedBallot(multiple)
        await verify(page, portal.origin, ballot, hash)
        await expect(page.getByText(hash, {exact: true})).toHaveCount(2)
        if (multiple) await expect(page.getByText("Charlie Example", {exact: true})).toBeVisible()
        await page.getByRole("link", {name: "Back", exact: true}).click()
        await expect(page).toHaveURL(/\/start/)
        await expect(page.getByRole("heading", {name: "Step 1: Import your ballot"})).toBeVisible()
    })
}

test("a mismatched ballot ID conceals selections after a valid verification", async ({
    page,
    portal,
}) => {
    const {ballot, hash} = await signedBallot()
    await verify(page, portal.origin, ballot, hash)
    await page.getByRole("link", {name: "Back", exact: true}).click()
    await upload(page, ballot, hash)
    const changed = (hash.startsWith("0") ? "1" : "0") + hash.slice(1)
    await page.getByRole("textbox", {name: "Ballot ID", exact: true}).fill(changed)
    await page.getByRole("button", {name: "Next", exact: true}).click()
    await expect(page.getByText("Does’t match the decoded ballot ID")).toBeVisible()
    await expect(page.getByText("Alice Example", {exact: true})).toHaveCount(0)
    await expect(page.getByRole("heading", {name: "Verify your ballot selections"})).toHaveCount(0)
})

for (const invalid of ["signature", "multi-signature", "json"] as const) {
    test(`invalid ${invalid} is rejected after a valid import and permits recovery`, async ({
        page,
        portal,
    }) => {
        const {ballot, hash} = await signedBallot(invalid === "multi-signature")
        await verify(page, portal.origin, ballot, hash)
        await page.getByRole("link", {name: "Back", exact: true}).click()
        const broken = structuredClone(ballot)
        // Flip one decoded signature byte while preserving valid base64 encoding.
        const signature = Buffer.from(broken.voter_ballot_signature!, "base64")
        signature[signature.length - 1] ^= 1
        broken.voter_ballot_signature = signature.toString("base64")
        await page.getByTestId("drop-input-file").setInputFiles({
            name: "broken.json",
            mimeType: "application/json",
            buffer: Buffer.from(invalid === "json" ? "{invalid JSON" : JSON.stringify(broken)),
        })
        await expect(page.getByRole("alert")).toBeVisible()
        await expect(page.getByRole("button", {name: "Next", exact: true})).toBeDisabled()
        await upload(page, ballot, hash)
        await expect(page.getByRole("alert")).toBeHidden()
        await page.getByRole("button", {name: "Next", exact: true}).click()
        await expect(page.getByText("Alice Example", {exact: true})).toBeVisible()
    })
}

test("mobile import and verification preserve the ballot ID and selected candidate", async ({
    page,
    portal,
}) => {
    await page.setViewportSize({width: 390, height: 844})
    const {ballot, hash} = await signedBallot()
    await verify(page, portal.origin, ballot, hash)
    await expect(page.getByText(hash, {exact: true})).toHaveCount(2)
})

test("reloading a confirmation route returns to import without stale ballot contents", async ({
    page,
    portal,
}) => {
    const {ballot, hash} = await signedBallot()
    await verify(page, portal.origin, ballot, hash)
    await page.reload()
    await expect(page).toHaveURL(/\/start/)
    await expect(page.getByRole("textbox", {name: "Ballot ID", exact: true})).toHaveValue("")
    await expect(page.getByRole("button", {name: "Next", exact: true})).toBeDisabled()
})

test("verified selections expose no accessibility violations", async ({page, portal}) => {
    const {ballot, hash} = await signedBallot()
    await verify(page, portal.origin, ballot, hash)
    const violations = await scanPage(page)
    if (violations.length) {
        expect(violations).toEqual([
            {id: "listitem", impact: "serious", targets: [[".candidate-item"]]},
        ])
    }
    expect(violations).toEqual([])
})

test("authentication-disabled verifier checks a signed ballot without private services", async ({
    page,
    portal,
}) => {
    portal.settings.DISABLE_AUTH = true
    const {ballot, hash} = await signedBallot()
    await verify(page, portal.origin, ballot, hash)
    expect(portal.oidc.authorizations).toEqual([])
    expect(portal.oidc.tokenRequests).toEqual([])
    expect(portal.graphql.calls).toEqual([])
})

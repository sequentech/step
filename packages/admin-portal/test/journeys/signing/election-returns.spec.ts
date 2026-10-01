// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect} from "../fixtures"
import {
    ANA,
    ER_BASE,
    ER_REQUEST_ID,
    ER_ROLES,
    LOCAL_NOTE,
    POST,
    SPAIN,
    displayName,
    electionReturns,
    expectRole,
    fixture,
    openCertificate,
    openRequest,
    sha256,
    signInAs,
    signingDialog,
    approvalLeaf,
    verifyApproval,
    verifyDetachedCms,
} from "./data"

/** window.open would leave the portal; the blob URLs it was given are the contract. */
async function recordOpenedWindows(page: Page) {
    await page.addInitScript(() => {
        const opened: string[] = []
        Object.assign(window, {openedWindows: opened})
        window.open = (url?: string | URL) => {
            opened.push(String(url))
            return null
        }
    })
    return () => page.evaluate(() => (window as unknown as {openedWindows: string[]}).openedWindows)
}

test.use({roles: ER_ROLES})

test("the third SBEI checks the returns' hash and completes them with a PAdES revision", async ({
    page,
    portal,
}) => {
    const opened = await recordOpenedWindows(page)
    signInAs(portal, ANA)
    const {server, state} = electionReturns(portal)
    const panel = await openRequest(page, portal)
    await expect(panel.getByTestId("signing-status")).toHaveText("Waiting · 2 of 3")
    await expect(panel.locator(`[data-signer="${ANA.username}"]`)).toContainText(
        `${displayName(ANA)} (you)`
    )
    await panel.getByRole("button", {name: "Sign", exact: true}).click()

    const dialog = signingDialog(page)
    await expect(dialog).toHaveAccessibleName("Sign the election returns")
    const documentCard = dialog.getByTestId("signing-document")
    const hash = sha256(ER_BASE)
    await expect(documentCard).toContainText(`Election returns, ${POST}, ${SPAIN}.pdf`)
    await expect(documentCard).toContainText(`PDF · SHA-256 ${hash.slice(0, 8)}…${hash.slice(-6)}`)
    // Open the document: the base is downloaded and its hash checked before it is shown.
    await documentCard.getByRole("button", {name: "Open the document", exact: true}).click()
    await expect.poll(opened).toHaveLength(1)
    expect((await opened())[0]).toMatch(/^blob:/)
    await expect(documentCard.getByRole("alert")).toHaveCount(0)
    expect(server.requests.get(ER_REQUEST_ID)?.documentUrl).toBeTruthy()
    expect(
        portal.s3.requests.filter(({key}) => key.endsWith(`${ER_REQUEST_ID}/base.pdf`))
    ).toHaveLength(1)
    await expect(dialog.getByText(LOCAL_NOTE)).toBeVisible()
    const next = dialog.getByRole("button", {name: "Continue", exact: true})
    await expect(next).toBeDisabled()
    await dialog.getByRole("checkbox", {name: "I have checked the election returns"}).check()
    await next.click()

    await expect(dialog.getByText(LOCAL_NOTE)).toBeVisible()
    await openCertificate(page, ANA.certificate)
    const checks = dialog.getByRole("list", {name: "Certificate checks"})
    await expect(checks.locator('[data-check="registered"]')).toHaveText(
        "First use: it will be registered to you"
    )
    await expect(checks.locator('[data-check="trusted-issuer"]')).toHaveText(
        "Issued by a trusted issuer (Test PNPKI Root CA)"
    )
    await dialog.getByRole("button", {name: "Sign", exact: true}).click()
    await expect(dialog.getByText("All 3 signatures are in.", {exact: true})).toBeVisible()
    await expect(dialog.getByText(LOCAL_NOTE)).toBeVisible()
    await dialog.getByRole("button", {name: "Done", exact: true}).click()

    await expect(panel.getByTestId("signing-status")).toHaveText("Done · 3 of 3")
    await expect(panel.getByRole("button", {name: "Download signed PDF"})).toBeVisible()
    await expect(panel.getByRole("button", {name: "Transmit results"})).toBeVisible()

    // The server prepared revision 3 and Ana's CMS signs its digest; her payload signature verifies.
    const prepared = portal.graphql.callsTo("SigningPdfPrepare")
    expect(prepared).toHaveLength(1)
    const approve = portal.graphql.callsTo("SigningApprove")
    expect(approve.map(({variables}) => variables.revision)).toEqual([3])
    const {variables} = approve[0]
    expect(verifyApproval(variables, state.canonicalPayload)).toBe(true)
    const cms = Buffer.from(String(variables.pdf_cms_b64), "base64")
    expect(verifyDetachedCms(cms, state.preparedDigests[0], approvalLeaf(variables))).toBe(true)
    expect(verifyDetachedCms(cms, Buffer.alloc(32, 1), approvalLeaf(variables))).toBe(false)
    expect(server.received(ER_REQUEST_ID)).toEqual([
        expect.objectContaining({
            person: ANA,
            algorithm: fixture(ANA.certificate).algorithm,
            payloadVerified: true,
            cmsVerified: true,
            revision: 3,
        }),
    ])
    expect(state.revisions).toBe(3)
    expect(state.executionResult).toMatchObject({revision: 3})
    // First use registered Ana's certificate to her.
    expect(server.registrations.get(fixture(ANA.certificate).fingerprintSha256)?.person).toBe(ANA)
    expectRole(portal, "GetSigningRequests", "signing-requests-read")
    // Before the panel knows the request's action, it reads it as a request reader.
    const reads = portal.graphql.callsTo("SigningGetRequest")
    expect(reads.map((call) => call.headers["x-hasura-role"])).toEqual([
        "signing-requests-read",
        ...reads.slice(1).map(() => "sign-generate-election-returns"),
    ])
    for (const operation of ["SigningCheckCertificate", "SigningPdfPrepare", "SigningApprove"])
        expectRole(portal, operation, "sign-generate-election-returns")
})

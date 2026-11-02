// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Page} from "@playwright/test"
import {test, expect} from "../fixtures"
import {
    ANA,
    ER_REQUEST_ID,
    ER_ROLES,
    JOSE,
    LOCAL_NOTE,
    MARIA,
    REVOKED_SERIALS,
    actionError,
    displayName,
    electionReturns,
    expectRole,
    fixture,
    openCertificate,
    openRequest,
    signInAs,
    signingDialog,
} from "./data"

test.use({roles: ER_ROLES})

/** From the panel to the Certificate step of the election returns. */
async function toCertificateStep(page: Page, panel: Awaited<ReturnType<typeof openRequest>>) {
    await panel.getByRole("button", {name: "Sign", exact: true}).click()
    const dialog = signingDialog(page)
    await dialog.getByRole("checkbox", {name: "I have checked the election returns"}).check()
    await dialog.getByRole("button", {name: "Continue", exact: true}).click()
    await expect(dialog.getByText(LOCAL_NOTE)).toBeVisible()
    return dialog
}

const checkItem = (dialog: ReturnType<typeof signingDialog>, id: string) =>
    dialog.getByRole("list", {name: "Certificate checks"}).locator(`[data-check="${id}"]`)

test("the file itself refuses a wrong password, and the failure is logged without it", async ({
    page,
    portal,
}) => {
    signInAs(portal, ANA)
    electionReturns(portal)
    const dialog = await toCertificateStep(page, await openRequest(page, portal))
    await openCertificate(page, ANA.certificate, "Wrong-2028")
    await expect(dialog.getByText("Wrong password. Check it and try again.")).toBeVisible()
    await expect(dialog.getByTestId("signing-certificate")).toHaveCount(0)
    await expect.poll(() => portal.graphql.callsTo("SigningOpenFailure").length).toBe(1)
    expect(portal.graphql.callsTo("SigningOpenFailure")[0].variables).toEqual({
        request_id: ER_REQUEST_ID,
        file_name: ANA.certificate,
        reason: "wrong-password",
    })
    expectRole(portal, "SigningOpenFailure", "sign-generate-election-returns")
    expect(portal.graphql.callsTo("SigningCheckCertificate")).toHaveLength(0)

    // The right password opens it.
    await dialog.getByTestId("certificate-password").fill(fixture(ANA.certificate).password)
    await dialog.getByRole("button", {name: "Open certificate", exact: true}).click()
    await expect(checkItem(dialog, "valid-now")).toHaveText("Valid today")
    const sent = JSON.stringify(portal.graphql.calls.map(({variables}) => variables))
    expect(sent).not.toContain("Wrong-2028")
    expect(sent).not.toContain(fixture(ANA.certificate).password)
})

test("an expired certificate can't sign, and a revocation found at approval refuses one", async ({
    page,
    portal,
}) => {
    signInAs(portal, ANA)
    const {server} = electionReturns(portal)
    const dialog = await toCertificateStep(page, await openRequest(page, portal))
    const sign = dialog.getByRole("button", {name: "Sign", exact: true})

    // The server's dry run finds the certificate expired (valid until 2025).
    await openCertificate(page, "ramon-garcia-expired.p12")
    await expect(checkItem(dialog, "valid-now")).toHaveText("Not valid today")
    await expect(checkItem(dialog, "valid-now")).toHaveAttribute("data-ok", "false")
    await expect(dialog.getByText("This certificate can't sign this request.")).toBeVisible()
    await expect(sign).toBeDisabled()

    // Another file passes the dry run, but the issuer's list revokes it before the approval.
    await dialog.getByRole("button", {name: "Choose another file", exact: true}).click()
    await openCertificate(page, "carmen-villanueva-revoked.p12")
    await expect(checkItem(dialog, "not-revoked")).toHaveAttribute("data-ok", "true")
    for (const serial of REVOKED_SERIALS) server.revokedSerials.add(serial)
    await sign.click()
    await expect(dialog.getByText("The server refused the signature.")).toBeVisible()
    await expect(checkItem(dialog, "not-revoked")).toHaveText(
        "Revoked, or no current revocation list to check it"
    )
    await expect(checkItem(dialog, "not-revoked")).toHaveAttribute("data-ok", "false")
    await expect(sign).toBeDisabled()
    // The approval was verified, then refused with the failed check.
    expect(server.received(ER_REQUEST_ID)).toEqual([
        expect.objectContaining({person: ANA, payloadVerified: true, cmsVerified: true}),
    ])
    expect(server.requests.get(ER_REQUEST_ID)?.approvals).toHaveLength(2)
})

test("Jose can't sign with Maria's certificate, nor sign twice (R1)", async ({page, portal}) => {
    signInAs(portal, JOSE)
    const {server} = electionReturns(portal, [MARIA])
    const dialog = await toCertificateStep(page, await openRequest(page, portal))
    const sign = dialog.getByRole("button", {name: "Sign", exact: true})

    await openCertificate(page, MARIA.certificate)
    await expect(checkItem(dialog, "registered-to-other")).toHaveText(
        `Registered to ${displayName(MARIA)}`
    )
    await expect(
        dialog.getByText(
            "This certificate can't sign for you. Use the certificate on your own security token."
        )
    ).toBeVisible()
    await expect(sign).toBeDisabled()

    // His own certificate passes; the server finds his key already signed meanwhile.
    await dialog.getByRole("button", {name: "Choose another file", exact: true}).click()
    await openCertificate(page, JOSE.certificate)
    await expect(checkItem(dialog, "registered")).toHaveText(/^First use/)
    server.once("SigningApprove", () =>
        actionError(422, "already-signed", "this key already signed the request")
    )
    await sign.click()
    await expect(dialog.getByTestId("signing-closed")).toHaveText(
        "You have already signed this request."
    )
    await expect(sign).toBeDisabled()
    expect(server.requests.get(ER_REQUEST_ID)?.approvals).toHaveLength(1)
})

test("a request cancelled while signing can't be signed (request-closed)", async ({
    page,
    portal,
}) => {
    signInAs(portal, ANA)
    const {server, state} = electionReturns(portal)
    const panel = await openRequest(page, portal)
    const dialog = await toCertificateStep(page, panel)
    await openCertificate(page, ANA.certificate)
    // A recount changes the returns while Ana signs: the approval arrives too late.
    server.once("SigningApprove", () => {
        state.status = "cancelled"
        state.cancelReason = "payload-changed"
        return actionError(409, "request-closed", "the request is not waiting", {
            status: "cancelled",
        })
    })
    await dialog.getByRole("button", {name: "Sign", exact: true}).click()
    await expect(dialog.getByTestId("signing-closed")).toHaveText(
        "This request was cancelled: What it signs changed. Signatures given for it no longer count. Start it again to sign the current version."
    )
    await expect(dialog.getByRole("button", {name: "Sign", exact: true})).toBeDisabled()
    await dialog.getByRole("button", {name: "Cancel", exact: true}).click()
    await expect(panel.getByTestId("signing-status")).toHaveText("Cancelled")
    await expect(panel.getByRole("button", {name: "Sign", exact: true})).toHaveCount(0)
})

test("a stale PDF revision is prepared again and signed", async ({page, portal}) => {
    signInAs(portal, ANA)
    const {server, state} = electionReturns(portal, [MARIA])
    const dialog = await toCertificateStep(page, await openRequest(page, portal))
    await openCertificate(page, ANA.certificate)
    // Jose signs on another laptop between Ana's prepare and her approval.
    server.once("SigningApprove", () => {
        server.signElsewhere(ER_REQUEST_ID, JOSE)
        return actionError(409, "stale-revision", "the prepared revision is stale")
    })
    await dialog.getByRole("button", {name: "Sign", exact: true}).click()
    await expect(dialog.getByText("All 3 signatures are in.", {exact: true})).toBeVisible()
    expect(portal.graphql.callsTo("SigningPdfPrepare")).toHaveLength(2)
    expect(
        portal.graphql.callsTo("SigningApprove").map(({variables}) => variables.revision)
    ).toEqual([2, 3])
    expect(server.received(ER_REQUEST_ID)).toEqual([
        expect.objectContaining({
            person: ANA,
            revision: 3,
            payloadVerified: true,
            cmsVerified: true,
        }),
    ])
    expect(state.revisions).toBe(3)
    expect(state.status).toBe("executed")
})

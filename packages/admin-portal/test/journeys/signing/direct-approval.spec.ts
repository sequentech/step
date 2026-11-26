// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {SigningRequestStatus, SigningSignerStatus} from "../../../src/lib/signing/types"
import type {Page} from "@playwright/test"
import {test, expect, type AdminPortal} from "../fixtures"
import {ALICE_ID, VOTER_TAB_ROLES, mockVoters, openVoters, rowAction} from "../access/data"
import {
    MARIA,
    JOSE,
    ELECTION_ID,
    SPAIN_ID,
    POST,
    SigningServer,
    SigningRequestState,
    displayName,
    expectRole,
    mockSignaturesTab,
    mockSigningEvent,
    openRequest,
    sbeiRoles,
    signInAs,
} from "./data"

const requestId = "f1000000-0000-4000-8000-000000000082"
const publication = "f2000000-0000-4000-8000-000000000082"
const code = "7F3A-91C2"
const profiles = [
    {
        mode: "hsm-demo-raw",
        action: "initialize-voting",
        title: "Initialization",
        operation: "SigningApproveInitializeDirect",
        field: "signingApproveInitializeDirect",
    },
    {
        mode: "hsm-demo-direct-action",
        action: "close-voting",
        title: "Closing",
        operation: "SigningApproveActionDirect",
        field: "signingApproveActionDirect",
    },
] as const

test.use({
    directHsm: true,
    roles: sbeiRoles(
        "election-event-signatures-tab",
        "signing-requests-read",
        "sign-initialize-voting",
        "sign-close-voting"
    ),
})

async function openReview(page: Page, portal: AdminPortal, profile: (typeof profiles)[number]) {
    signInAs(portal, MARIA)
    const server = new SigningServer(portal)
    const state = server.add({
        id: requestId,
        action: profile.action,
        required: 2,
        requestedBy: JOSE,
        code,
        signers: [MARIA, JOSE],
        subject:
            profile.action === "initialize-voting"
                ? {publication_id: publication}
                : {channels: ["ONLINE"], from: ["ONLINE=OPEN"]},
    })
    mockSigningEvent(portal)
    mockSignaturesTab(portal, {
        rules: [],
        capacities: {},
        checks: [],
        issuers: [],
        certificates: [],
        crls: [],
        requests: () => [state.row],
    })
    const data = state.panel(MARIA.userId)
    data.operational_signing_mode = profile.mode
    portal.graphql.on("SigningGetRequest", () => ({data: {signingGetRequest: {panel: data}}}))
    const drawer = await openRequest(page, portal, `${profile.title} · ${POST}`)
    const launcher = drawer.getByRole("button", {name: "Approve", exact: true})
    await launcher.click()
    const dialog = page.getByRole("dialog", {name: /^Sign /})
    await expect(dialog.getByText("Review", {exact: true})).toBeVisible()
    return {
        data,
        dialog,
        launcher,
        drawer,
        sign: dialog.getByRole("button", {name: "Sign", exact: true}),
    }
}

for (const [index, profile] of profiles.entries()) {
    test(`${profile.mode} reviews then shows only the recorded approval at ${index === 0 ? "mobile" : "desktop"} width`, async ({
        page,
        portal,
    }) => {
        await page.setViewportSize(
            index === 0 ? {width: 375, height: 667} : {width: 1280, height: 800}
        )
        const {data, dialog, launcher, drawer, sign} = await openReview(page, portal, profile)
        await expect(dialog.locator(".MuiStepLabel-label")).toHaveText(["Review", "Signed"])
        await expect(dialog).toContainText(displayName(MARIA))
        await expect(dialog).toContainText(code)
        await expect(dialog).toContainText(profile.title)
        await expect(dialog.locator('input[type="file"], input[type="password"]')).toHaveCount(0)
        await expect(drawer).not.toContainText("digital certificate")
        expect(
            await dialog.evaluate(
                (el) =>
                    el.scrollWidth <= el.clientWidth &&
                    el.getBoundingClientRect().right <= innerWidth
            )
        ).toBe(true)
        await dialog.screenshot({path: test.info().outputPath("review.png")})
        await page.keyboard.press("Escape")
        await expect(dialog).toHaveCount(0)
        await expect(launcher).toBeFocused()
        expect(portal.graphql.callsTo(profile.operation)).toHaveLength(0)
        await launcher.press("Enter")

        let release: () => void = () => {}
        const pending = new Promise<void>((resolve) => {
            release = resolve
        })
        portal.graphql.on(profile.operation, async () => {
            await pending
            return {data: {[profile.field]: {request_id: requestId}}}
        })
        await sign.focus()
        await page.keyboard.press("Enter")
        await page.keyboard.press("Enter")
        await expect.poll(() => portal.graphql.callsTo(profile.operation).length).toBe(1)
        await expect(sign).toHaveAttribute("aria-busy", "true")
        await page.keyboard.press("Escape")
        await expect(dialog).toBeVisible()
        release()
        await expect(dialog.getByRole("button", {name: "Check status"})).toBeEnabled()
        await dialog.getByRole("button", {name: "Cancel", exact: true}).click()
        await drawer.getByRole("button", {name: "Close the request panel"}).click()
        await page.getByRole("button", {name: `${profile.title} · ${POST}`, exact: true}).click()
        await launcher.click()
        const check = dialog.getByRole("button", {name: "Check status"})
        await expect(check).toBeEnabled()
        await expect(dialog.getByText("Your approval was recorded.")).toHaveCount(0)
        // The acknowledgement alone and a different member's row are insufficient.
        data.signers[1].signed_at = "2026-07-18T08:30:00Z"
        await check.click()
        await expect(check).toBeEnabled()
        await expect(dialog.getByText("Your approval was recorded.")).toHaveCount(0)
        data.signers[0].signed_at = "2026-07-18T09:47:00Z"
        data.signers[0].status = SigningSignerStatus.Signed
        data.count = 2
        data.signers[1].status = SigningSignerStatus.Signed
        data.request.status = SigningRequestStatus.Completed
        await check.click()
        await expect(dialog.getByText("Your approval was recorded.")).toBeVisible()
        await expect(dialog).toContainText("09:47")
        await expect(sign).toHaveCount(0)
        await dialog.screenshot({path: test.info().outputPath("signed.png")})
        await dialog.getByRole("button", {name: "Done"}).press("Enter")
        await expect(dialog).toHaveCount(0)
        expect(portal.graphql.callsTo(profile.operation).map((call) => call.variables)).toEqual([
            {request_id: requestId, payload_sha256: data.request.payload_sha256},
        ])
        expectRole(portal, profile.operation, `sign-${profile.action}`)
        for (const op of [
            "SigningApprove",
            "SigningCheckCertificate",
            profiles[1 - index].operation,
        ])
            expect(portal.graphql.callsTo(op)).toHaveLength(0)
    })
}

test("a changed digest blocks the sign selected in Review", async ({page, portal}) => {
    const profile = profiles[0]
    const {data, dialog, sign} = await openReview(page, portal, profile)
    data.request.payload_sha256 = "ff".repeat(32)
    await sign.click()
    await expect(sign).toBeDisabled()
    await expect(dialog.getByRole("button", {name: "Done"})).toHaveCount(0)
    expect(portal.graphql.callsTo(profile.operation)).toHaveLength(0)
})

test("a failed signing attempt stays on Review and can retry after an unsigned readback", async ({
    page,
    portal,
}) => {
    const profile = profiles[1]
    const {data, dialog, sign} = await openReview(page, portal, profile)
    portal.graphql.once(profile.operation, () => ({
        errors: [{message: "Synthetic signing service unavailable"}],
    }))
    await sign.click()
    await expect(dialog.getByRole("alert")).toBeVisible()
    await expect(sign).toBeEnabled()
    portal.graphql.on(profile.operation, () => {
        data.signers[0].signed_at = "2026-07-18T09:47:00Z"
        data.signers[0].status = SigningSignerStatus.Signed
        data.count = 1
        return {data: {[profile.field]: {request_id: requestId}}}
    })
    await sign.click()
    await expect(dialog.getByText("Your approval was recorded.")).toBeVisible()
    expect(portal.graphql.callsTo(profile.operation)).toHaveLength(2)
})

test("a lost signing response is recovered from the same member's saved approval", async ({
    page,
    portal,
}) => {
    const profile = profiles[1]
    const {data, dialog, sign} = await openReview(page, portal, profile)
    portal.graphql.on(profile.operation, () => {
        data.signers[0].signed_at = "2026-07-18T09:47:00Z"
        data.signers[0].status = SigningSignerStatus.Signed
        data.count = 1
        portal.graphql.once("SigningGetRequest", () => ({
            errors: [{message: "Synthetic read failure"}],
        }))
        return {errors: [{message: "Synthetic reply lost after recording"}]}
    })
    await sign.click()
    await dialog.getByRole("button", {name: "Check status"}).click()
    await expect(dialog.getByText("Your approval was recorded.")).toBeVisible()
    expect(portal.graphql.callsTo(profile.operation)).toHaveLength(1)
})

test.describe("Outside the registered Portal origin", () => {
    test.use({directHsm: false})

    test("the same reviewed request cannot sign", async ({page, portal}) => {
        const {dialog, sign} = await openReview(page, portal, profiles[0])
        await expect(sign).toBeDisabled()
        await expect(dialog.getByRole("alert")).toBeVisible()
        expect(portal.graphql.callsTo(profiles[0].operation)).toHaveLength(0)
    })
})

test.describe("Manually verify identity", () => {
    test.use({
        roles: [...VOTER_TAB_ROLES, "voter-manually-verify", "sign-authorize-manual-verification"],
    })

    test("reviews the exact voter and records an HSM approval before the document can be issued", async ({
        page,
        portal,
    }) => {
        signInAs(portal, MARIA)
        mockVoters(portal)
        let state: SigningRequestState
        let signedAt: string | null = null
        portal.graphql.on("ManualVerification", ({variables}) => {
            state = new SigningRequestState(
                {
                    id: requestId,
                    action: "authorize-manual-verification",
                    code,
                    required: 2,
                    requestedBy: MARIA,
                    signers: [MARIA, JOSE],
                    areaId: SPAIN_ID,
                    subject: {
                        voter_id: variables.voterId,
                        initiation_id: variables.initiationId,
                        requester_id: MARIA.userId,
                        post_id: ELECTION_ID,
                        area_id: SPAIN_ID,
                        secret_attributes: [],
                    },
                },
                portal.now
            )
            return {
                data: {
                    get_manual_verification_pdf: {
                        document_id: null,
                        status: "awaiting-signatures",
                        signing_request: {id: requestId},
                    },
                },
            }
        })
        portal.graphql.on("SigningGetRequest", () => {
            const data = state.panel(MARIA.userId)
            data.operational_signing_mode = "hsm-demo-direct-action"
            if (signedAt) {
                data.signers[0].signed_at = signedAt
                data.signers[0].status = SigningSignerStatus.Signed
                data.count = 1
            }
            return {data: {signingGetRequest: {panel: data}}}
        })
        portal.graphql.on("SigningApproveActionDirect", () => {
            signedAt = new Date(portal.now).toISOString()
            return {data: {signingApproveActionDirect: {request_id: requestId}}}
        })
        await openVoters(page, portal)
        await rowAction(page, "Manually verify identity")
        await page
            .getByRole("dialog")
            .getByRole("button", {name: "Request verification document", exact: true})
            .click()
        const dialog = page.getByRole("dialog", {name: /^Sign /})
        await expect(dialog.locator(".MuiStepLabel-label")).toHaveText(["Review", "Signed"])
        await expect(dialog.getByRole("table", {name: "Details"})).toContainText(ALICE_ID)
        const initiation = portal.graphql.callsTo("ManualVerification")[0].variables.initiationId
        await expect(dialog.getByRole("table", {name: "Details"})).toContainText(String(initiation))
        await expect(dialog.getByTestId("signing-code")).toHaveText(code)
        await dialog.getByRole("button", {name: "Sign", exact: true}).click()
        await expect(dialog.getByText("Your approval was recorded.")).toBeVisible()
        await dialog.getByRole("button", {name: "Done"}).click()
        await expect(page.getByTestId("signing-status")).toHaveText("Waiting · 1 of 2")
        expect(portal.graphql.callsTo("SigningApproveActionDirect")).toHaveLength(1)
        expectRole(portal, "SigningApproveActionDirect", "sign-authorize-manual-verification")
        expect(portal.graphql.callsTo("ManualVerification")).toHaveLength(1)
        expect(portal.graphql.callsTo("FetchDocument")).toHaveLength(0)
        expect(portal.graphql.callsTo("ChangeApplicationStatus")).toHaveLength(0)
    })
})

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const assert = require("node:assert/strict")
const fs = require("node:fs")
const {createHmac} = require("node:crypto")
const {chromium} = require("playwright")

const uuid = /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/
const loopback = (value) => {
    const url = new URL(value)
    if (
        url.protocol !== "https:" ||
        !["127.0.0.1", "[::1]"].includes(url.hostname) ||
        !url.port ||
        url.username ||
        url.password
    ) {
        throw new Error("The live rehearsal accepts only explicit HTTPS loopback endpoints")
    }
    return url
}

function oneTimePassword(seed) {
    assert.match(seed, /^[A-Z2-7]{16,128}$/)
    let bits = 0
    let buffer = 0
    const bytes = []
    for (const digit of seed) {
        buffer = (buffer << 5) | "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567".indexOf(digit)
        bits += 5
        if (bits >= 8) {
            bits -= 8
            bytes.push((buffer >> bits) & 255)
        }
    }
    const counter = Buffer.alloc(8)
    counter.writeBigUInt64BE(BigInt(Math.floor(Date.now() / 30000)))
    const mac = createHmac("sha1", Buffer.from(bytes)).update(counter).digest()
    const number = mac.readUInt32BE(mac[mac.length - 1] & 15) & 0x7fffffff
    return String(number % 1000000).padStart(6, "0")
}

function readFixture() {
    const file = process.env.STEP_SYNTHETIC_SIGNING_FIXTURE
    if (process.env.STEP_CONFIRM_SYNTHETIC_SIGNING !== "SOFTHSM_LOOPBACK_ONLY" || !file) {
        throw new Error(
            "Set STEP_CONFIRM_SYNTHETIC_SIGNING=SOFTHSM_LOOPBACK_ONLY and provide a disposable fixture file"
        )
    }
    const fixture = JSON.parse(fs.readFileSync(file, "utf8"))
    assert.equal(fixture.synthetic, true)
    assert.equal(fixture.provider, "softhsm-test")
    for (const key of ["tenantId", "eventId", "electionId", "requestId"])
        assert.match(fixture[key], uuid)
    assert.equal(fixture.officers.length, 2)
    for (const officer of fixture.officers) {
        assert.match(officer.id, uuid)
        assert.ok(
            typeof officer.username === "string" && officer.username.startsWith("hsm-fixture-")
        )
        assert.ok(typeof officer.password === "string" && officer.password.length > 0)
        assert.match(officer.totpSeed, /^[A-Z2-7]{16,128}$/)
    }
    assert.notEqual(fixture.officers[0].id, fixture.officers[1].id)
    assert.notEqual(fixture.officers[0].username, fixture.officers[1].username)
    assert.ok(fixture.officers.some((officer) => officer.needsSetup === true))
    assert.ok(typeof fixture.requestRow === "string" && fixture.requestRow.length > 0)
    const portal = loopback(fixture.portalUrl)
    const issuer = loopback(fixture.keycloakUrl)
    assert.equal(portal.pathname, "/")
    assert.ok(!issuer.search && !issuer.hash)
    // Match the isolated broker's exact pinned callback and issuer admission.
    assert.equal(portal.origin, "https://127.0.0.1:13443")
    assert.equal(issuer.origin, "https://127.0.0.1:18443")
    const requestPage = loopback(fixture.requestPageUrl)
    assert.equal(requestPage.origin, portal.origin)
    assert.ok(
        [
            `/sequent_backend_election_event/${fixture.eventId}`,
            `/sequent_backend_election/${fixture.electionId}`,
        ].some((base) => [base, base + "/show", base + "/edit"].includes(requestPage.pathname))
    )
    assert.ok(!requestPage.hash)
    return {fixture, portal, issuer, requestPage}
}

const hostedLogin = async (page, issuer, tenant, officer, gold) => {
    const expectedRealm = issuer.pathname.replace(/\/$/, "") + "/realms/tenant-" + tenant
    await page.waitForURL(
        (url) => url.origin === issuer.origin && url.pathname.startsWith(expectedRealm + "/")
    )
    const password = page.locator("input[name=password]").first()
    const username = page.locator("input[name=username]").first()
    await password.waitFor()
    if ((await username.count()) > 0 && (await username.isEditable())) {
        await username.fill(officer.username)
    }
    await password.fill(officer.password)
    await page.locator("button[type=submit], input[type=submit]").first().click()
    if (gold) {
        const otp = page.locator("input[name=otp], input[name=otpCode]").first()
        await otp.waitFor()
        await otp.fill(oneTimePassword(officer.totpSeed))
        await page.locator("button[type=submit], input[type=submit]").first().click()
    }
}

async function openRequest(page, fixture) {
    const row = page.getByRole("row").filter({hasText: fixture.requestRow})
    // Some event pages open on their default tab after normal login.
    if (!(await row.isVisible())) {
        const signatures = page.getByRole("tab", {name: "Signatures", exact: true})
        if (await signatures.count()) await signatures.click()
        const requests = page.getByRole("tab", {name: "Requests", exact: true})
        if (await requests.count()) await requests.click()
    }
    await row.waitFor()
    await row.click()
    const subject = page.getByTestId("signing-status")
    await subject.waitFor()
    return subject
}

async function main() {
    const {fixture, portal, issuer, requestPage} = readFixture()
    const browser = await chromium.launch({
        executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
        headless: true,
    })
    try {
        for (const [i, officer] of fixture.officers.entries()) {
            const context = await browser.newContext({
                ignoreHTTPSErrors: true,
                serviceWorkers: "block",
                locale: "en-US",
            })
            try {
                const allowed = new Set([portal.origin, issuer.origin])
                await context.route("**/*", async (route) => {
                    const url = new URL(route.request().url())
                    if (allowed.has(url.origin) && url.protocol === "https:") await route.continue()
                    else await route.abort()
                })
                await context.routeWebSocket("**/*", (ws) => ws.close())
                const page = await context.newPage()
                page.setDefaultTimeout(35000)
                // The signed-in fixture's tenant settings must have test-only endpoints and no remote APIs.
                const login = page.goto(requestPage.href)
                await hostedLogin(page, issuer, fixture.tenantId, officer, false)
                await login.catch(() => undefined)
                await page.waitForURL((url) => url.origin === portal.origin)
                const quorum = await openRequest(page, fixture)
                const requestState = []
                const mutations = []
                const watch = async (response) => {
                    if (
                        new URL(response.url()).origin !== portal.origin ||
                        new URL(response.url()).pathname !== "/v1/graphql"
                    )
                        return
                    const query = response.request().postDataJSON()
                    if (
                        query?.operationName === "SigningGetRequest" &&
                        query?.variables?.request_id === fixture.requestId
                    ) {
                        const body = await response.json()
                        const panel = body?.data?.signingGetRequest?.panel
                        if (
                            panel?.request?.id === fixture.requestId &&
                            panel.request.tenant_id === fixture.tenantId &&
                            panel.request.election_event_id === fixture.eventId &&
                            panel.request.election_id === fixture.electionId &&
                            panel.request.action === "initialize-voting" &&
                            panel.request.required === 2 &&
                            panel.operational_signing_mode === "hsm-required"
                        )
                            requestState.push(panel)
                    }
                    if (
                        [
                            "OperationalInitializeSetup",
                            "OperationalInitializeStart",
                            "OperationalGoldReturn",
                        ].includes(query?.operationName)
                    ) {
                        // Deliberately do not retain body, Gold authorization credentials, or response data.
                        mutations.push(query.operationName)
                    }
                }
                page.on("response", (response) => void watch(response).catch(() => undefined))
                await page.getByRole("button", {name: "Sign", exact: true}).click()
                let dialog = page.getByRole("dialog", {name: "Review approval"})
                await dialog
                    .getByText(
                        "Demo only: signatures use a software test token, not a hardware device."
                    )
                    .waitFor()
                assert.equal(await dialog.locator('input[type="file"]').count(), 0)
                if (officer.needsSetup) {
                    const setup = dialog.getByRole("button", {name: "Set up signing", exact: true})
                    await setup.click()
                    await hostedLogin(page, issuer, fixture.tenantId, officer, true)
                    await page.waitForURL((url) => url.origin === portal.origin)
                    dialog = page.getByRole("dialog", {name: "Review approval"})
                    await dialog.getByRole("button", {name: "Set up signing", exact: true}).click()
                    await dialog.getByText(/Signing setup is pending/).waitFor()
                    assert.equal(
                        await dialog.getByRole("button", {name: "Approve", exact: true}).count(),
                        0
                    )
                    assert.equal(
                        mutations.filter((operation) => operation === "OperationalInitializeStart")
                            .length,
                        0
                    )
                    console.log(
                        "Observed managed first-use setup without an approval. Waiting for the fixture issuer."
                    )
                }
                // An isolated fixture issuer must attach the approved synthetic certificate while pending.
                const approve = dialog.getByRole("button", {name: "Approve", exact: true})
                await approve.waitFor({timeout: 30000})
                await approve.click()
                await hostedLogin(page, issuer, fixture.tenantId, officer, true)
                await page.waitForURL(
                    (url) =>
                        url.origin === portal.origin &&
                        url.pathname !== "/operational-signing/return"
                )
                dialog = page.getByRole("dialog", {name: "Review approval"})
                await dialog.getByText("Your approval was recorded.").waitFor()
                await dialog.getByText(`${i + 1} of 2 approvals recorded`).waitFor()
                await dialog.getByRole("button", {name: "Close", exact: true}).click()
                if (i === 0) {
                    assert.match(await quorum.innerText(), /Waiting/)
                } else {
                    await page.waitForFunction(() =>
                        document
                            .querySelector('[data-testid="signing-status"]')
                            ?.textContent?.includes("Done")
                    )
                }
                const observed = requestState.findLast(
                    (panel) =>
                        panel.count === i + 1 &&
                        panel.request.status === (i === 0 ? "waiting" : "executed")
                )
                assert.ok(
                    observed,
                    "The exact managed Initialize request did not reach its expected durable state"
                )
                assert.ok(
                    observed.signers.some(
                        (signer) => signer.user_id === officer.id && signer.signed_at
                    )
                )
                if (i === 1)
                    assert.equal(
                        new Set(observed.signers.filter((s) => s.signed_at).map((s) => s.user_id))
                            .size,
                        2
                    )
                assert.equal(
                    mutations.filter((operation) => operation === "OperationalInitializeStart")
                        .length,
                    1
                )
                assert.equal(
                    mutations.filter((operation) => operation === "OperationalGoldReturn").length,
                    1
                )
            } catch {
                throw new Error(
                    `Officer ${i + 1} did not reach the expected managed approval state. Verify the isolated fixture's login, certificate issuer, request, and status.`
                )
            } finally {
                await context.close()
            }
        }
        console.log(
            "PASS live isolated portal: two distinct officers approved one Initialize request; it completed after quorum."
        )
    } finally {
        await browser.close()
    }
}
main().catch((error) => {
    console.error(error.message)
    process.exitCode = 1
})

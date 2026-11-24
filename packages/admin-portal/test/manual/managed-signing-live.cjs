// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

const assert = require("node:assert/strict")
const fs = require("node:fs")
const {createHmac} = require("node:crypto")
const {setTimeout: sleep} = require("node:timers/promises")
const {freshOtpStep, makeStageGate, matchesWitnessPanel} = require("./managed-signing-witness.cjs")
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

function oneTimePassword(seed, step) {
    assert.ok(/^[A-Z2-7]{16,128}$/.test(seed), "Invalid fixture OTP seed")
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
    counter.writeBigUInt64BE(BigInt(step))
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
    let fixture
    try {
        fixture = JSON.parse(fs.readFileSync(file, "utf8"))
    } catch {
        throw new Error("Could not read the private synthetic signing fixture")
    }
    assert.equal(fixture.synthetic, true)
    assert.equal(fixture.provider, "softhsm-test")
    for (const key of ["tenantId", "eventId", "electionId", "requestId"])
        assert.match(fixture[key], uuid)
    assert.match(fixture.payloadSha256, /^[0-9a-f]{64}$/)
    assert.equal(fixture.officers.length, 2)
    for (const officer of fixture.officers) {
        assert.match(officer.id, uuid)
        assert.ok(
            typeof officer.username === "string" && officer.username.startsWith("hsm-fixture-")
        )
        assert.ok(typeof officer.password === "string" && officer.password.length > 0)
        assert.ok(/^[A-Z2-7]{16,128}$/.test(officer.totpSeed), "Invalid fixture OTP seed")
    }
    assert.notEqual(fixture.officers[0].id, fixture.officers[1].id)
    assert.notEqual(fixture.officers[0].username, fixture.officers[1].username)
    assert.ok(
        fixture.officers[0].totpSeed !== fixture.officers[1].totpSeed,
        "Use distinct fixture OTP seeds"
    )
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

const hostedLogin = async (page, issuer, tenant, officer, gold, otpState) => {
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
        let value = freshOtpStep(Date.now(), otpState.step)
        while (value.waitMs) {
            await sleep(value.waitMs)
            value = freshOtpStep(Date.now(), otpState.step)
        }
        otpState.step = value.step
        await otp.fill(oneTimePassword(officer.totpSeed, value.step))
        await page.locator("button[type=submit], input[type=submit]").first().click()
    }
}

async function openRequest(page, fixture) {
    const row = page.getByRole("row").filter({hasText: fixture.requestRow})
    // Some event pages open on their default tab after normal login.
    if (!(await row.isVisible())) {
        const signatures = page.getByRole("tab", {
            name: "Signatures",
            exact: true,
        })
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
    const witnessDir = process.env.STEP_SIGNING_WITNESS_DIR
    const witnessSeconds = Number(process.env.STEP_SIGNING_WITNESS_TIMEOUT_SECONDS ?? "60")
    assert.ok(
        Number.isInteger(witnessSeconds) && witnessSeconds >= 5 && witnessSeconds <= 180,
        "Fixture observer timeout must be 5 to 180 seconds"
    )
    const witness = witnessDir
        ? makeStageGate(witnessDir, {timeoutMs: witnessSeconds * 1000})
        : undefined
    await witness?.("before")
    const browser = await chromium.launch({
        executablePath: process.env.CHROMIUM_EXECUTABLE_PATH,
        headless: true,
    })
    try {
        for (const [i, officer] of fixture.officers.entries()) {
            const otpState = {}
            let stage = "sign-in"
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
                stage = "inspect exact request"
                const requestState = []
                let changedScope = false
                const mutations = []
                const queryFrom = (request) => {
                    const url = new URL(request.url())
                    if (url.origin !== portal.origin || url.pathname !== "/v1/graphql")
                        return undefined
                    return request.postDataJSON()
                }
                const watch = async (response) => {
                    const query = queryFrom(response.request())
                    if (
                        query?.operationName === "SigningGetRequest" &&
                        query?.variables?.request_id === fixture.requestId
                    ) {
                        try {
                            const panel = (await response.json())?.data?.signingGetRequest?.panel
                            if (matchesWitnessPanel(panel, fixture)) requestState.push(panel)
                            else changedScope = true
                        } catch {
                            changedScope = true
                        }
                    }
                }
                page.on("response", (response) => void watch(response).catch(() => undefined))
                context.on("request", (request) => {
                    try {
                        const operation = queryFrom(request)?.operationName
                        if (
                            [
                                "OperationalInitializeSetup",
                                "OperationalInitializeStart",
                                "OperationalGoldReturn",
                            ].includes(operation)
                        )
                            // Record names only; don't retain any authorization material.
                            mutations.push(operation)
                    } catch {
                        // Non-JSON requests do not carry a GraphQL operation.
                    }
                })
                const waitForPanel = async (count, status, since = 0) => {
                    const deadline = performance.now() + 12000
                    while (performance.now() <= deadline) {
                        assert.ok(
                            !changedScope,
                            "The exact fixture request scope or digest changed"
                        )
                        const panel = requestState
                            .slice(since)
                            .findLast(
                                (value) => value.count === count && value.request.status === status
                            )
                        if (panel) return panel
                        await sleep(40)
                    }
                    throw new Error("The exact fixture request did not reach its expected state")
                }
                const quorum = await openRequest(page, fixture)
                await waitForPanel(i, "waiting")
                await page.getByRole("button", {name: "Sign", exact: true}).click()
                let dialog = page.getByRole("dialog", {name: "Review approval"})
                await dialog
                    .getByText(
                        "Demo only: signatures use a software test token, not a hardware device."
                    )
                    .waitFor()
                assert.equal(await dialog.locator('input[type="file"]').count(), 0)
                if (officer.needsSetup) {
                    stage = "first-use setup"
                    const setup = dialog.getByRole("button", {
                        name: "Set up signing",
                        exact: true,
                    })
                    await setup.click()
                    await hostedLogin(page, issuer, fixture.tenantId, officer, true, otpState)
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
                    if (witness) {
                        stage = "observe setup without an approval"
                        await witness(i === 0 ? "first-setup" : "second-setup")
                    }
                }
                stage = "submit one reviewed approval"
                // An isolated fixture issuer must attach the approved synthetic certificate while pending.
                const approve = dialog.getByRole("button", {
                    name: "Approve",
                    exact: true,
                })
                await approve.waitFor({timeout: 30000})
                await waitForPanel(i, "waiting")
                await approve.click()
                await hostedLogin(page, issuer, fixture.tenantId, officer, true, otpState)
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
                    assert.match(await quorum.innerText(), /^\s*Waiting\s*·\s*1 of 2\s*$/)
                } else {
                    await page.waitForFunction(
                        () =>
                            document
                                .querySelector('[data-testid="signing-status"]')
                                ?.textContent?.trim() === "Done · 2 of 2"
                    )
                }
                const observed = await waitForPanel(i + 1, i === 0 ? "waiting" : "executed")
                assert.ok(
                    observed.signers.some(
                        (signer) => signer.user_id === officer.id && signer.signed_at
                    )
                )
                if (i === 1)
                    assert.deepEqual(
                        new Set(observed.signers.filter((s) => s.signed_at).map((s) => s.user_id)),
                        new Set(fixture.officers.map((o) => o.id))
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
                if (witness) {
                    stage = "observe approval and worker effect"
                    await witness(i === 0 ? "first-approved" : "second-approved")
                }
                if (i === 1) {
                    stage = "refresh the completed request"
                    const beforeRefresh = [...mutations]
                    for (let repeat = 0; repeat < 2; repeat++) {
                        const since = requestState.length
                        await page.reload()
                        const settled = await openRequest(page, fixture)
                        await waitForPanel(2, "executed", since)
                        assert.match(await settled.innerText(), /^\s*Done\s*·\s*2 of 2\s*$/)
                        assert.equal(
                            await page.getByRole("button", {name: "Sign", exact: true}).count(),
                            0
                        )
                    }
                    await witness?.("after-refresh")
                    // Catch requests scheduled by the remount, even if they never get a response.
                    await sleep(750)
                    assert.ok(!changedScope, "The exact fixture request scope or digest changed")
                    assert.deepEqual(
                        mutations,
                        beforeRefresh,
                        "A completed request sent another mutation"
                    )
                }
            } catch {
                throw new Error(
                    `Officer ${i + 1} failed at stage: ${stage}. Check the isolated fixture without logging credentials.`
                )
            } finally {
                await context.close()
            }
        }
        console.log(
            witness
                ? "PASS browser and observer stages: retain the separate SQL evidence for device receipts and the single worker effect."
                : "PASS browser flow only: two officers reached quorum; no independent device/effect witness was supplied."
        )
    } finally {
        await browser.close()
    }
}
main().catch((error) => {
    console.error(error.message)
    process.exitCode = 1
})

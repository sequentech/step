// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {test, expect, eventPath, realm, IDS, electionFixture, type Portal} from "./fixtures"
import type {Page} from "@playwright/test"
import {readFile} from "node:fs/promises"
import {loadCore} from "@sequentech/ui-test-kit/wasm/node"

test("authenticated voter loads their published ballot through OIDC, GraphQL and S3", async ({
    page,
    portal,
}) => {
    await page.goto(`${portal.origin}${eventPath}?lang=en`)
    await expect(page.getByText("Community Council", {exact: true})).toBeVisible()
    await expect(page.getByRole("button", {name: /click to vote/i})).toBeEnabled()
    expect(portal.oidc.authorizations[0]).toMatchObject({
        realm,
        params: {client_id: "voting-portal", code_challenge_method: "S256", ui_locales: "en"},
    })
    expect(portal.oidc.tokenRequests[0]).toMatchObject({
        grantType: "authorization_code",
        pkce: "valid",
        status: 200,
    })
    expect(portal.graphql.callsTo("GetVoterStatus")).toHaveLength(1)
    expect(
        portal.s3.requests
            .filter(({bucket}) => bucket === "private")
            .map(({key}) => key)
            .sort()
    ).toEqual(["election.json", "event.json", "summary.json"])
})

async function review(page: Page, portal: Portal, preview = false) {
    await page.goto(`${portal.origin}${preview ? portal.previewPath : eventPath + "?lang=en"}`)
    await page.getByRole("button", {name: /click to vote/i}).click()
    if (preview)
        await page.getByRole("button", {name: "I understand that my vote will not be cast"}).click()
    await page.getByRole("button", {name: "Start Voting", exact: true}).click()
    await page.getByRole("checkbox", {name: /Alice Example/}).check()
    await expect(page.getByRole("checkbox", {name: /Alice Example/})).toBeChecked()
    await expect(page.getByRole("checkbox", {name: /Bob Example/})).not.toBeChecked()
    await page.getByRole("button", {name: "Next", exact: true}).click()
    await expect(page.getByRole("heading", {name: /^Review your ballot/})).toBeVisible()
    await expect(page.getByText("Alice Example", {exact: true})).toBeVisible()
}

test("cast sends an encrypted ballot whose ID is shown on confirmation", async ({page, portal}) => {
    await review(page, portal)
    const ballotId = (await page.getByText(/^Your Ballot ID:/).textContent())?.match(
        /[0-9a-f]{64}/
    )?.[0]
    expect(ballotId).toMatch(/^[0-9a-f]{64}$/)
    await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
    await expect(page).toHaveURL(/\/confirmation/)
    await expect(page.getByTestId("ballot-id").first()).toHaveText(ballotId!)
    const [cast] = portal.graphql.callsTo("InsertCastVote")
    expect(cast.variables).toMatchObject({electionId: IDS.election, ballotId})
    const ballot = JSON.parse(String(cast.variables.content))
    expect(ballot).toMatchObject({version: 2, config: IDS.style})
    expect(ballot.contests).toHaveLength(1)
    expect(cast.headers.authorization).toBe(`Bearer ${portal.oidc.lastIssued()?.access_token}`)
})

test("preview completes a demo vote without an authenticated cast", async ({page, portal}) => {
    portal.data = electionFixture({demo: true})
    portal.publish()
    await review(page, portal, true)
    await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
    await expect(page).toHaveURL(/\/confirmation/)
    expect(portal.graphql.callsTo("InsertCastVote")).toEqual([])
    expect(portal.oidc.authorizations).toEqual([])
})

test("audit download independently decodes to the selected candidate and never casts", async ({
    page,
    portal,
}) => {
    await review(page, portal)
    await page.getByRole("button", {name: "Audit ballot", exact: true}).click()
    await page.getByRole("button", {name: "Yes, discard my ballot to audit"}).click()
    await expect(page).toHaveURL(/\/audit/)
    const pending = page.waitForEvent("download")
    await page.getByRole("button", {name: "Download", exact: true}).click()
    const download = await pending
    const bytes = await readFile((await download.path())!, "utf8")
    const core = await loadCore()
    const decoded = core.decode_auditable_ballot_js(JSON.parse(bytes))
    expect(decoded).toMatchObject([
        {contest_id: IDS.contest, is_blank_ballot: false, is_explicit_invalid: false},
    ])
    expect(decoded[0].choices.filter(({selected}) => selected >= 0)).toEqual([
        {id: IDS.alice, selected: 0, write_in_text: null},
    ])
    expect(portal.graphql.callsTo("InsertCastVote")).toEqual([])
})

for (const code of ["AreaNotFound", "CheckStatusFailed", "InsertFailedExceedsAllowedRevotes"]) {
    test(`cast ${code} keeps the ballot available and retries successfully`, async ({
        page,
        portal,
    }) => {
        await review(page, portal)
        portal.graphql.once("InsertCastVote", () => ({
            errors: [{message: "Cannot cast this ballot", extensions: {code}}],
        }))
        await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
        await expect(page.getByRole("alert")).toBeVisible()
        await expect(page).toHaveURL(/\/review/)
        await expect(page.getByText("Alice Example", {exact: true})).toBeVisible()
        await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
        await expect(page).toHaveURL(/\/confirmation/)
        const calls = portal.graphql.callsTo("InsertCastVote")
        expect(calls).toHaveLength(2)
        expect(calls[1].variables).toEqual(calls[0].variables)
    })
}

test("aborted cast remains on review and can retry the same ballot", async ({page, portal}) => {
    await review(page, portal)
    portal.graphql.once("InsertCastVote", () => ({abort: "connectionfailed"}))
    await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
    await expect(page.getByRole("alert")).toBeVisible()
    await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
    await expect(page).toHaveURL(/\/confirmation/)
    const calls = portal.graphql.callsTo("InsertCastVote")
    expect(calls).toHaveLength(2)
    expect(calls[1].variables).toEqual(calls[0].variables)
})

test("an expired publication URL is renewed once and downloaded without credentials", async ({
    page,
    context,
    portal,
}) => {
    await context.addCookies([
        {url: portal.origin, name: "private-session", value: "must-not-reach-s3"},
    ])
    portal.s3.override(({key}) => key === "event.json", {status: 403}, 1)
    await page.goto(`${portal.origin}${eventPath}?lang=en`)
    await expect(page.getByRole("button", {name: /click to vote/i})).toBeEnabled()
    expect(portal.graphql.callsTo("GetVoterStatus")).toHaveLength(2)
    expect(portal.s3.requestsFor("event.json").map(({status}) => status)).toEqual([403, 200])
    for (const request of portal.s3.requests.filter(({bucket}) => bucket === "private")) {
        expect(request.headers).not.toHaveProperty("authorization")
        expect(request.headers).not.toHaveProperty("cookie")
        expect(request.headers).not.toHaveProperty("referer")
    }
})

test("repeated expired URLs stop after one renewal and show a data error", async ({
    page,
    portal,
}) => {
    portal.s3.override(({key}) => key === "event.json", {status: 403})
    await page.goto(`${portal.origin}${eventPath}?lang=en`)
    await expect(page.getByText(/There was a problem fetching the data/)).toBeVisible()
    expect(portal.graphql.callsTo("GetVoterStatus")).toHaveLength(2)
    expect(portal.s3.requestsFor("event.json").map(({status}) => status)).toEqual([403, 403])
    await expect(page.getByRole("button", {name: /click to vote/i})).toHaveCount(0)
})

test("a publication from another event never offers a ballot", async ({page, portal}) => {
    portal.s3.putJson("private", "event.json", {
        ...portal.data.event,
        id: "20000000-0000-4000-8000-000000000002",
    })
    await page.goto(`${portal.origin}${eventPath}?lang=en`)
    await expect(page.getByText(/There was a problem fetching the data/)).toBeVisible()
    expect(portal.graphql.callsTo("GetVoterStatus")).toHaveLength(1)
    await expect(page.getByRole("button", {name: /click to vote/i})).toHaveCount(0)
})

test("login hints are forwarded to Keycloak and removed from the callback URL", async ({
    page,
    portal,
}) => {
    await page.goto(
        `${portal.origin}${eventPath}/login?lang=en&login_hint__username=synthetic-voter&login_hint__district=North`
    )
    await expect(page.getByRole("button", {name: /click to vote/i})).toBeEnabled()
    expect(portal.oidc.authorizations[0].params).toMatchObject({
        login_hint: "synthetic-voter",
        login_hint__username: "synthetic-voter",
        login_hint__district: "North",
    })
    expect(page.url()).not.toContain("login_hint")
    expect(portal.oidc.authorizations[0].params.redirect_uri).not.toContain("login_hint")
})

test("kiosk and enrollment select their intended OIDC client and endpoint", async ({
    page,
    portal,
}) => {
    await page.goto(`${portal.origin}${eventPath}/enroll?kiosk&lang=en`)
    await expect(page.getByRole("button", {name: /click to vote/i})).toBeEnabled()
    expect(portal.oidc.authorizations[0]).toMatchObject({
        kind: "registration",
        params: {client_id: "voting-portal-kiosk"},
    })
    expect(portal.oidc.authorizations[0].params.redirect_uri).toContain("/login?")
})

test("gold-level casting reauthenticates and preserves the selected ballot", async ({
    page,
    portal,
}) => {
    portal.data = electionFixture({gold: true})
    portal.publish()
    await review(page, portal)
    const ballotId = (await page.getByText(/^Your Ballot ID:/).textContent())?.match(
        /[0-9a-f]{64}/
    )?.[0]
    await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
    await expect(page).toHaveURL(/\/confirmation/)
    expect(portal.oidc.authorizations.map(({requestedAcr}) => requestedAcr)).toEqual([
        undefined,
        "gold",
    ])
    expect(portal.oidc.tokenRequests.every(({pkce}) => pkce === "valid")).toBe(true)
    expect(portal.graphql.callsTo("InsertCastVote")[0].variables.ballotId).toBe(ballotId)
})

test("expired access tokens leave the portal through the configured OIDC logout", async ({
    page,
    portal,
}) => {
    await page.goto(`${portal.origin}${eventPath}?lang=en`)
    await expect(page.getByRole("button", {name: /click to vote/i})).toBeEnabled()
    portal.now += 901000
    await page.clock.fastForward(901000)
    await expect.poll(() => portal.oidc.logouts.length).toBe(1)
    expect(portal.oidc.logouts[0].params.post_logout_redirect_uri).toBe(
        `${portal.origin}${eventPath}`
    )
    await expect(page.getByRole("button", {name: "Sign in", exact: true})).toBeVisible()
})

test("mobile voter can choose, review and cast with no accessibility violations", async ({
    page,
    portal,
}) => {
    await page.setViewportSize({width: 390, height: 844})
    await review(page, portal)
    const {scanPage} = await import("@sequentech/ui-test-kit/adapters/axe")
    const violations = await scanPage(page)
    await page.getByRole("button", {name: "Cast ballot", exact: true}).click()
    await expect(page).toHaveURL(/\/confirmation/)
    await expect(page.getByTestId("ballot-id").last()).toBeVisible()
    expect(
        violations.filter(
            ({id, targets}) =>
                id !== "button-name" ||
                JSON.stringify(targets) !== JSON.stringify([["#lang-button"]])
        )
    ).toEqual([])
    test.fail(
        true,
        "Mobile Header language button has no accessible name; pinned in HeaderPrimaryMobile story too"
    )
    expect(violations).toEqual([])
})

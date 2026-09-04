// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, Page} from "@playwright/test"

export interface CastBallotOptions {
    loginUrl: string
    // Every column from the voter's CSV row (a "password" PIN/credential,
    // plus whichever match-attributes — a voter-id username, a date of
    // birth, or some combination — the realm's login form is configured to
    // ask for).
    credentials: Record<string, string>
    // Only candidates whose visible name matches are selected.
    candidatesPattern?: string
}

// Encrypting and casting the ballot are the slow steps of the flow — WASM
// encryption per contest plus the cast round-trip — so the waits around them
// get a longer budget than ordinary screen transitions.
const castTimeoutMs = 60_000

// The demo dialog is shown every time a demo event lands on the election
// list; a real event never shows it. Wait for whichever appears first
// instead of paying a fixed timeout on non-demo events.
async function settleOnElectionList(page: Page): Promise<void> {
    const electionItem = page.locator(".election-item").first()
    const demoAccept = page.getByRole("button", {name: "I accept my vote will Not be cast"})
    await expect(electionItem.or(demoAccept).first()).toBeVisible()
    if (await demoAccept.isVisible()) {
        await demoAccept.click()
        await expect(electionItem).toBeVisible()
    }
}

// The login form's non-credential fields are realm-configurable
// match-attributes (a voter-id username, a date of birth, or some
// combination) — fill in whichever of the voter's CSV columns the form
// actually asks for, rather than assuming a fixed username/password shape.
// The credential field itself renders either as a plain password input, or —
// when the realm's credential-input-policy is "structured" (e.g. a segmented
// PIN) — as a JS-enhanced #structured-password input that mirrors typed
// digits into the real (now hidden) password field.
async function login(page: Page, credentials: Record<string, string>): Promise<void> {
    for (const [field, value] of Object.entries(credentials)) {
        if (field === "password") {
            continue
        }
        const input = page.locator(`input[name="${field}"]`)
        if ((await input.count()) > 0) {
            await input.fill(value)
        }
    }

    const structuredPin = page.locator("#structured-password")
    const plainPassword = page.locator('input[name="password"]')
    await expect(structuredPin.or(plainPassword).first()).toBeVisible()
    if (await structuredPin.isVisible()) {
        await structuredPin.click()
        await structuredPin.pressSequentially(credentials.password)
    } else {
        await plainPassword.fill(credentials.password)
    }

    await page.locator("#kc-login").click()
}

async function selectCandidates(page: Page, candidatesPattern?: string): Promise<void> {
    const contestTitles = page.locator(".contest-title")
    await expect(contestTitles.first()).toBeVisible()
    const contestCount = await contestTitles.count()
    for (let contest = 0; contest < contestCount; contest++) {
        const title = contestTitles.nth(contest)
        const min = Number((await title.getAttribute("data-min")) ?? 0)
        const max = Number((await title.getAttribute("data-max")) ?? 1)
        let candidates = title.locator("xpath=..").locator(".candidate-item")
        if (candidatesPattern) {
            candidates = candidates.filter({hasText: new RegExp(candidatesPattern)})
        }
        const available = await candidates.count()
        // Deterministic selection: enough to satisfy the contest minimum, at
        // least one (an empty selection turns the whole ballot into the
        // blank-ballot flow), never more than the maximum or than what is
        // on screen.
        const toSelect = Math.min(Math.max(min, 1), max, available)
        for (let i = 0; i < toSelect; i++) {
            await candidates.nth(i).click()
        }
    }
}

async function voteElection(
    page: Page,
    electionIndex: number,
    candidatesPattern?: string
): Promise<string> {
    await page.locator(".election-item").nth(electionIndex).locator(".click-to-vote-button").click()

    // .start-voting-button always renders (disabled or not), so it is a
    // stable anchor for "the start screen has loaded" even when a demo
    // dialog (no real key-ceremony public key attached to this election)
    // covers it first.
    const startVoting = page.locator(".start-voting-button")
    const demoAccept = page.getByRole("button", {name: "I accept my vote will Not be cast"})
    await expect(startVoting.or(demoAccept).first()).toBeVisible()
    if (await demoAccept.isVisible()) {
        await demoAccept.click()
        await expect(startVoting).toBeVisible()
    }

    // The eligibility declaration checkbox only renders when the election's
    // security confirmation policy is MANDATORY, and gates the start button
    // while unchecked.
    const eligibilityCheckbox = page.locator(".security-confirmation-checkbox input[type=checkbox]")
    if (await eligibilityCheckbox.isVisible()) {
        await eligibilityCheckbox.check()
    }
    await startVoting.click()

    // The ballot is paginated one group of contests per page — select on the
    // current page, advance, and repeat until Next lands on the review
    // screen (cast-ballot-button) instead of another page of contests.
    const contestTitle = page.locator(".contest-title").first()
    const castButton = page.locator(".cast-ballot-button")
    for (;;) {
        const previousTitle = await contestTitle.textContent().catch(() => null)
        await selectCandidates(page, candidatesPattern)
        await page.locator(".next-button").click()
        // Next either swaps in a new page of contests (same component tree,
        // so the title changes) or — on the last page — triggers WASM
        // re-encryption before navigating to Review. Poll for one of those
        // actually happening rather than a single toBeVisible check, which
        // can trivially pass while the previous, still-mounted page's title
        // is technically "visible" during that transition.
        await expect
            .poll(
                async () => {
                    if (await castButton.isVisible()) {
                        return true
                    }
                    const currentTitle = await contestTitle.textContent().catch(() => null)
                    return currentTitle !== null && currentTitle !== previousTitle
                },
                {timeout: castTimeoutMs}
            )
            .toBe(true)
        if (await castButton.isVisible()) {
            break
        }
    }
    await castButton.click()

    // Casting may first open a confirmation dialog, depending on the election
    // event's cast_vote_confirm_modal setting — wait for whichever of the
    // dialog or the confirmation screen shows up.
    const ballotIdEl = page.getByTestId("ballot-id").first()
    const confirmCast = page.getByRole("button", {name: "Yes, I want to CAST my vote"})
    await expect(ballotIdEl.or(confirmCast).first()).toBeVisible({timeout: castTimeoutMs})
    if (await confirmCast.isVisible()) {
        await confirmCast.click()
    }

    // The non-empty ballot id on the confirmation screen is the success
    // criterion: it only renders once the ballot has actually been cast.
    await expect(ballotIdEl).toBeVisible({timeout: castTimeoutMs})
    const ballotId = ((await ballotIdEl.textContent()) ?? "").trim()
    expect(ballotId).not.toBe("")
    return ballotId
}

// Drives one voter through the full flow — login, then for every election in
// the event: select candidates, review, cast, and read the ballot id off the
// confirmation screen. Returns the ballot id(s). No explicit logout: each
// Playwright test runs in a fresh browser context, so sessions don't leak
// between voters.
export async function castBallotAsVoter(page: Page, options: CastBallotOptions): Promise<string[]> {
    // The flow matches on English button labels, so pin the portal language
    // rather than depending on the browser locale.
    const url = new URL(options.loginUrl)
    url.searchParams.set("lang", "en")
    await page.goto(url.toString())

    await login(page, options.credentials)

    await settleOnElectionList(page)
    const electionCount = await page.locator(".election-item .click-to-vote-button").count()
    expect(electionCount).toBeGreaterThan(0)

    const ballotIds: string[] = []
    for (let election = 0; election < electionCount; election++) {
        ballotIds.push(await voteElection(page, election, options.candidatesPattern))
        if (election < electionCount - 1) {
            // Finish navigates back to the election list while voting remains
            // open; on the last election the cast is already confirmed, so
            // there is nothing left to navigate to.
            await page.locator(".finish-button").click()
            await settleOnElectionList(page)
        }
    }
    return ballotIds
}

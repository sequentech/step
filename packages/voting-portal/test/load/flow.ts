// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {expect, Page} from "@playwright/test"

export interface CastBallotOptions {
    loginUrl: string
    username: string
    password: string
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
    await page.getByRole("button", {name: "Start Voting"}).click()

    await selectCandidates(page, candidatesPattern)

    await page.locator(".next-button").click()
    await page.locator(".cast-ballot-button").click()

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

    await page.locator("input[name=username]").fill(options.username)
    await page.locator("input[name=password]").fill(options.password)
    await page.locator("[type=submit]").click()

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
